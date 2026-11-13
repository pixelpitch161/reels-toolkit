//! The seed search: `StartSearch`, `StopSearch` and `SetFilters`.
//!
//! **How a search runs.** `StartSearch` starts one worker thread for its generation. The worker makes the draft's
//! fight file and set-up, then picks up where an earlier search of the same search key got to: the search it
//! replaces, when that was of the same key, or else the shortlist cache on disk (its seeds are fought again, since
//! fights are reproducible, and the list is sent at once). Then it calls `bf_story::search` for [`CHUNK`] seeds at a
//! time, merges each chunk into the shortlist it keeps (every fit and the best [`OTHERS`] of the rest) and sends a
//! `Shortlist` after each chunk. The seeds checked are always 1 to `next_seed - 1`, whatever the number of workers,
//! so the shortlist is the same for any number of them. While the preview is drawing, a chunk runs on half the
//! workers (at least one) so the preview keeps its frames; `PreviewBusy` and `SetWorkers` are read before each
//! chunk, so a change takes effect at the next chunk (a chunk is a fraction of a second).
//!
//! **How it ends.** `Full` at [`FITS`] fits; `Stopped` on `StopSearch` (within one fight per worker, or once the
//! cached seeds are fought again, at most `FITS + OTHERS` fights), once `config.search_limit` seeds are checked,
//! or when a fight cannot be fought (with an `Event::Problem` in words); silently when a newer `StartSearch`
//! replaces it, for another generation or the same one (its late answers are never sent: the new search's worker
//! waits, up to `STOP_WAIT`, for the old one to finish before it starts). A panic on the worker ends it with the
//! engine's `Event::Problem` and a `Stopped` list of what it had. The cache is saved after every eighth chunk and
//! whenever the search ends; a silenced search never saves over a cache that got further. A cache file that does
//! not hold together (more fits or others than a shortlist keeps, a seed twice, a seed past its next seed, a
//! `checked` that disagrees) is a miss.
//!
//! **Filters.** `SetFilters` for the running search's generation stores the filters; while the worker runs, it
//! sends the list again at its next seed (so only one thread ever sends a running search's lists, in order); once
//! it has ended, `handle` sends the list from memory itself. For any other generation the answer is an empty, idle
//! list, as the sample engine's. A list resent at the next seed is built inside `bf_story::search`'s progress
//! callback, under the story search's own lock: building it takes well under a millisecond (at most `FITS +
//! OTHERS` rows) and happens once per filter change, so the search's workers wait that long at most, once.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use bf_core::fightfile::FightFile;
use bf_sim::Setup;
use bf_story::{BeatSheet, Candidate, SearchEnd, SearchOpts};

use super::{Ctx, STOP_WAIT, convert, problem};
use crate::model::Gen;
use crate::model::draft::Draft;
use crate::model::shortlist::{FilterSpec, Finish, SearchState, Shortlist};
use crate::model::story::{AbilityMark, StoryView};
use crate::seam::{Command, Event};
use crate::store::shortlist_cache::{CachedShortlist, load_shortlist, save_shortlist};
use crate::wording;

/// Seeds a chunk of the search checks.
pub const CHUNK: u64 = 512;
/// The shortlist is full at this many fits.
pub const FITS: usize = 200;
/// How many fights that do not fit the shortlist keeps (the best of them): the engine's own `KEEP_MISFITS`.
pub const OTHERS: usize = bf_story::KEEP_MISFITS;
/// The cache is saved after every this many chunks (and when the search ends).
const SAVE_EVERY: u64 = 8;
/// The cache is kept per app version.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the interface shows of one candidate: the story's numbers, the ability names as shown ("Flutter tongue"),
/// each miss with a capital first letter ("No K.O.").
pub fn story_view(c: &Candidate, t_fight: f64) -> StoryView {
    let st = &c.story;
    StoryView {
        seed: c.seed,
        winner: st.winner,
        t_ko: st.t_ko,
        winner_hp: st.winner_hp,
        lead_changes: st.lead_changes,
        score: c.verdict.score,
        fits: c.verdict.fits,
        misses: c.verdict.misses.iter().map(|m| capital(m)).collect(),
        t_fight,
        lead: st.lead.iter().map(|&(t, d)| (t as f32, d as f32)).collect(),
        firsts: st
            .firsts
            .iter()
            .map(|(t, who, name)| AbilityMark {
                t: *t as f32,
                who: *who,
                name: wording::display_name(name),
            })
            .collect(),
    }
}

/// `text` with a capital first letter, the rest as it is.
fn capital(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The engine's filters for the interface's: `Close` is a close finish, `Comeback` a comeback, `show_outside`
/// shows the fights that do not fit.
pub fn filters(spec: &FilterSpec) -> bf_story::Filters {
    bf_story::Filters {
        winner: spec.winner,
        ko: Some(spec.ko),
        min_lead_changes: Some(spec.min_lead_changes),
        close_finish: spec.finish == Finish::Close,
        comeback: spec.finish == Finish::Comeback,
        ability: spec.ability.clone(),
        show_misfits: spec.show_outside,
    }
}

/// The search key of a fight file, as the cache is keyed: what feeds its fight (`FightFile::search_key`, with each
/// fighter's music counted when `bf_content::music_feeds_fight` says so).
pub fn search_key(ff: &FightFile) -> String {
    let feeds: Vec<bool> = ff.fighters.iter().map(|n| bf_content::music_feeds_fight(n)).collect();
    ff.search_key(&feeds)
}

/// The threads a chunk runs on: all of them, or half (at least one) while the preview is drawing.
fn chunk_workers(workers: usize, busy: bool) -> usize {
    if busy { workers / 2 } else { workers }.max(1)
}

/// Tests: a search of this generation panics after its first chunk, on purpose.
#[cfg(test)]
const PANICS: Gen = 9_999;

/// Why the engine refuses a fight's set-up, as a plain sentence. `field` is the `SimError`'s field (such as
/// `fighters[1]` or `rules.swat_cobras`); its message is left out, since it is written for the fight file's author
/// (field paths, kinds in code form), not for the screen.
fn unsearchable(field: &str, fighters: &[String]) -> String {
    let fighter = field
        .strip_prefix("fighters[")
        .and_then(|rest| rest.strip_suffix(']'))
        .and_then(|i| i.parse::<usize>().ok())
        .and_then(|i| fighters.get(i));
    let why = match fighter {
        Some(name) => format!("the engine cannot fight with {}", wording::display_name(name)),
        None => "the engine cannot use one of its settings".to_string(),
    };
    convert::plain(&format!("This fight cannot be searched yet: {why}"))
}

/// Why the search stopped at a fight that could not be fought (`SearchEnd::Failed`, whose words begin "seed N:"),
/// as a plain sentence.
fn failed_sentence(words: &str) -> String {
    let seed = words
        .strip_prefix("seed ")
        .and_then(|rest| rest.split(':').next())
        .and_then(|n| n.parse::<u64>().ok());
    match seed {
        Some(n) => format!(
            "The search stopped at seed {}, because that fight could not be fought. Try another handicap.",
            wording::thousands(n)
        ),
        None => "The search stopped, because a fight could not be fought. Try another handicap.".to_string(),
    }
}

/// Whether `ours` may be saved over `disk`: only when it got further (an older search never undoes a newer one's
/// progress).
fn fresher(ours: &CachedShortlist, disk: Option<&CachedShortlist>) -> bool {
    disk.is_none_or(|d| ours.next_seed > d.next_seed)
}

/// Whether `ours` may be saved over the cache read from disk: [`fresher`] than it, where a cache that does not
/// hold together ([`sound`]) counts as none.
fn fresher_than_disk(ours: &CachedShortlist, disk: Option<CachedShortlist>) -> bool {
    fresher(ours, disk.filter(sound).as_ref())
}

/// Whether a cached shortlist holds together: within the shortlist's bounds, no seed twice, every seed below its
/// next seed, and `checked` one less than it.
fn sound(cached: &CachedShortlist) -> bool {
    let mut seeds: Vec<u64> = cached.fit_seeds.iter().chain(&cached.other_seeds).copied().collect();
    seeds.sort_unstable();
    cached.next_seed > 0
        && cached.checked == cached.next_seed - 1
        && cached.fit_seeds.len() <= FITS
        && cached.other_seeds.len() <= OTHERS
        && seeds.windows(2).all(|w| w[0] != w[1])
        && seeds.iter().all(|&s| s > 0 && s < cached.next_seed)
}

/// A lock that a panic elsewhere cannot poison for good.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// What a search has found, shared by its worker and the handler.
struct Memory {
    filters: FilterSpec,
    /// The search key, once the worker has made the fight file.
    key: Option<String>,
    /// The shortlist as `bf_story::shortlist` keeps it: ranked, every fit and the best `OTHERS` of the rest.
    candidates: Vec<Candidate>,
    fits: usize,
    /// The next seed to check: seeds 1 to `next_seed - 1` are checked.
    next_seed: u64,
    state: SearchState,
    /// Threads the last chunk ran on.
    workers: usize,
    t_fight: f64,
    /// The worker is running: it, not the handler, sends this search's lists.
    running: bool,
}

impl Memory {
    fn new(filters: FilterSpec, workers: usize) -> Memory {
        Memory {
            filters,
            key: None,
            candidates: Vec::new(),
            fits: 0,
            next_seed: 1,
            state: SearchState::Searching,
            workers,
            t_fight: 0.0,
            running: true,
        }
    }

    /// The list as it stands, through the filters.
    fn list(&self) -> Shortlist {
        let keep = filters(&self.filters);
        Shortlist {
            rows: self
                .candidates
                .iter()
                .filter(|c| keep.keep(c))
                .map(|c| story_view(c, self.t_fight))
                .collect(),
            fits_total: u32::try_from(self.fits).unwrap_or(u32::MAX),
            checked: self.next_seed.saturating_sub(1),
            state: self.state,
            workers: self.workers,
        }
    }

    /// What the cache keeps of it.
    fn cached(&self) -> CachedShortlist {
        let (fits, others): (Vec<&Candidate>, Vec<&Candidate>) = self.candidates.iter().partition(|c| c.verdict.fits);
        CachedShortlist {
            next_seed: self.next_seed,
            checked: self.next_seed.saturating_sub(1),
            fit_seeds: fits.iter().map(|c| c.seed).collect(),
            other_seeds: others.iter().map(|c| c.seed).collect(),
        }
    }
}

/// One search: its generation, what it shares with its worker, and its flags.
struct Run {
    r#gen: Gen,
    memory: Arc<Mutex<Memory>>,
    /// Set by `StopSearch`, a newer search or the engine's ending: the worker stops at its next fight.
    stop: Arc<AtomicBool>,
    /// Set by a newer search: the worker sends nothing more.
    silent: Arc<AtomicBool>,
    /// Set by `SetFilters` while the worker runs: it sends the list again.
    resend: Arc<AtomicBool>,
    /// Set by the worker as it ends (even by a panic).
    done: Arc<AtomicBool>,
}

/// What this handler keeps between commands: the newest search.
#[derive(Default)]
pub struct State {
    run: Option<Run>,
}

/// Takes one command and returns at once; the search runs on its own thread.
pub fn handle(state: &mut State, ctx: &Ctx, command: Command) {
    match command {
        Command::StartSearch { r#gen, draft, filters } => start(state, ctx, r#gen, draft, filters),
        Command::StopSearch => {
            if let Some(run) = &state.run {
                run.stop.store(true, Ordering::SeqCst);
            }
        }
        Command::SetFilters { r#gen, filters } => {
            let list = match &state.run {
                Some(run) if run.r#gen == r#gen => {
                    let mut m = lock(&run.memory);
                    m.filters = filters;
                    if m.running {
                        run.resend.store(true, Ordering::SeqCst);
                        None
                    } else {
                        Some(m.list())
                    }
                }
                // No search for that generation: nothing found.
                _ => Some(Shortlist {
                    rows: Vec::new(),
                    fits_total: 0,
                    checked: 0,
                    state: SearchState::Idle,
                    workers: ctx.workers(),
                }),
            };
            if let Some(list) = list {
                ctx.emit(Event::Shortlist { r#gen, list });
            }
        }
        _ => {}
    }
}

/// Starts the search of `gen`, ending the one before it (silently when it was of another generation).
fn start(state: &mut State, ctx: &Ctx, r#gen: Gen, draft: Draft, filters: FilterSpec) {
    // Only the newest search sends lists, so the old one is stopped and silenced whatever its generation; its
    // memory is the new one's start when it was of the same search key (the same draft searched on).
    let before = state.run.as_ref().map(|old| {
        old.silent.store(true, Ordering::SeqCst);
        old.stop.store(true, Ordering::SeqCst);
        (old.memory.clone(), old.done.clone())
    });
    let run = Run {
        r#gen,
        memory: Arc::new(Mutex::new(Memory::new(filters, ctx.workers()))),
        stop: ctx.cancel_flag(),
        silent: Arc::new(AtomicBool::new(false)),
        resend: Arc::new(AtomicBool::new(false)),
        done: Arc::new(AtomicBool::new(false)),
    };
    let me = Worker {
        r#gen,
        memory: run.memory.clone(),
        stop: run.stop.clone(),
        silent: run.silent.clone(),
        resend: run.resend.clone(),
        done: run.done.clone(),
    };
    if ctx.spawn(
        "engine: search",
        "searching seeds",
        move |ctx| search_worker(&ctx, &me, &draft, before),
        problem,
    ) {
        state.run = Some(run);
    }
}

/// The worker's side of a [`Run`].
struct Worker {
    r#gen: Gen,
    memory: Arc<Mutex<Memory>>,
    stop: Arc<AtomicBool>,
    silent: Arc<AtomicBool>,
    resend: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
}

impl Worker {
    /// Whether the worker may still send.
    fn heard(&self, ctx: &Ctx) -> bool {
        !self.silent.load(Ordering::SeqCst) && !ctx.stopping()
    }

    /// Sends the list as it stands (unless the search was silenced). Either way a resend asked for is answered: a
    /// silenced search owes no list, so `finish` never waits on one.
    fn send(&self, ctx: &Ctx) {
        let list = {
            let m = lock(&self.memory);
            self.resend.store(false, Ordering::SeqCst);
            if !self.heard(ctx) {
                return;
            }
            m.list()
        };
        // checked once more right before sending: a newer search may have taken over meanwhile
        if self.heard(ctx) {
            ctx.emit(Event::Shortlist {
                r#gen: self.r#gen,
                list,
            });
        }
    }

    /// Ends the search as `state`: the cache is saved, the last list sent, and the handler sends from then on.
    fn finish(&self, ctx: &Ctx, state: SearchState) {
        let saved = {
            let mut m = lock(&self.memory);
            m.state = state;
            m.key.clone().filter(|_| m.next_seed > 1).map(|key| (key, m.cached()))
        };
        if let Some((key, cached)) = saved {
            let dir = ctx.config.paths.shortlists();
            // A silenced search may have been overtaken by a newer one of the same key: it never saves over a
            // cache that got further. A cache on disk that does not hold together counts as none.
            let newer = self.silent.load(Ordering::SeqCst)
                && !fresher_than_disk(&cached, load_shortlist(&dir, APP_VERSION, &key));
            if !newer {
                // The cache is a convenience: a search that cannot be saved still stands in memory.
                _ = save_shortlist(&dir, APP_VERSION, &key, &cached);
            }
        }
        self.send(ctx);
        // a SetFilters that came in while the last list was being sent is answered before the handler takes over
        loop {
            let mut m = lock(&self.memory);
            if self.resend.load(Ordering::SeqCst) {
                drop(m);
                self.send(ctx);
                continue;
            }
            m.running = false;
            return;
        }
    }

    /// The search could not go on: the words, then the list as it stands, stopped.
    fn fail(&self, ctx: &Ctx, words: String) {
        if self.heard(ctx) {
            ctx.emit(problem(words));
        }
        self.finish(ctx, SearchState::Stopped);
    }
}

/// Marks the worker done when it ends, even by a panic, so neither the handler nor a newer search waits on it. A
/// panic (whose `Event::Problem` the engine sends) also ends the list: what the search had, `Stopped`, so the
/// interface offers to search again.
struct Done<'a>(&'a Worker, &'a Ctx);

impl Drop for Done<'_> {
    fn drop(&mut self) {
        let Done(me, ctx) = *self;
        let list = {
            let mut m = lock(&me.memory);
            m.running = false;
            (std::thread::panicking() && me.heard(ctx)).then(|| {
                m.state = SearchState::Stopped;
                m.list()
            })
        };
        if let Some(list) = list {
            ctx.emit(Event::Shortlist { r#gen: me.r#gen, list });
        }
        me.done.store(true, Ordering::SeqCst);
    }
}

/// The search's thread.
fn search_worker(ctx: &Ctx, me: &Worker, draft: &Draft, before: Option<(Arc<Mutex<Memory>>, Arc<AtomicBool>)>) {
    let _done = Done(me, ctx);
    // The search it replaces ends within one fight per worker; it is waited for so its last answer comes first.
    if let Some((_, done)) = &before {
        let deadline = Instant::now() + STOP_WAIT;
        while !done.load(Ordering::SeqCst)
            && Instant::now() < deadline
            && !ctx.stopping()
            && !me.stop.load(Ordering::SeqCst)
        {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    let ff = match convert::fight_file(draft) {
        Ok(ff) => ff,
        Err(words) => return me.fail(ctx, words),
    };
    let base = match bf_sim::setup_from(&ff, bf_content::roster(), 0) {
        Ok(base) => base,
        Err(e) => return me.fail(ctx, unsearchable(&e.field, &ff.fighters)),
    };
    let sheet = BeatSheet::series(base.fighters);
    let key = search_key(&ff);
    let setup_for = |seed: u64| Setup { seed, ..base.clone() };
    {
        let mut m = lock(&me.memory);
        m.key = Some(key.clone());
        m.t_fight = ff.grid().t_fight;
    }
    // where an earlier search of this key got to
    let earlier = before.and_then(|(memory, _)| {
        let old = lock(&memory);
        (old.key.as_deref() == Some(key.as_str()) && old.next_seed > 1).then(|| (old.candidates.clone(), old.next_seed))
    });
    let resumed = earlier.or_else(|| {
        let cached = load_shortlist(&ctx.config.paths.shortlists(), APP_VERSION, &key).filter(sound)?;
        refight(ctx, me, &setup_for, &sheet, &cached).map(|c| (c, cached.next_seed))
    });
    // What was resumed is installed first, so a stop that came at once keeps it (and saves it again).
    let full = resumed.is_some_and(|(mut candidates, next_seed)| {
        bf_story::shortlist(&mut candidates);
        let mut m = lock(&me.memory);
        m.fits = candidates.iter().filter(|c| c.verdict.fits).count();
        m.candidates = candidates;
        m.next_seed = next_seed;
        m.fits >= FITS
    });
    if full {
        return me.finish(ctx, SearchState::Full);
    }
    if me.stop.load(Ordering::SeqCst) {
        return me.finish(ctx, SearchState::Stopped);
    }
    if lock(&me.memory).next_seed > 1 {
        me.send(ctx);
    }
    let progress = |_: bf_story::SearchProgress| {
        if me.resend.load(Ordering::SeqCst) {
            me.send(ctx);
        }
    };
    let mut chunks = 0u64;
    loop {
        let (from, fits) = {
            let m = lock(&me.memory);
            (m.next_seed, m.fits)
        };
        let checked = from - 1;
        let remaining = ctx
            .config
            .search_limit
            .map_or(CHUNK, |limit| limit.saturating_sub(checked));
        if remaining == 0 {
            return me.finish(ctx, SearchState::Stopped);
        }
        let workers = chunk_workers(ctx.workers(), ctx.preview_busy());
        let opts = SearchOpts {
            from,
            stop_fits: FITS - fits,
            max_seeds: Some(remaining.min(CHUNK)),
            workers,
        };
        let result = bf_story::search(&setup_for, &sheet, &opts, &progress, &me.stop);
        let full = {
            let mut m = lock(&me.memory);
            m.candidates.extend(result.candidates);
            bf_story::shortlist(&mut m.candidates);
            m.fits = m.candidates.iter().filter(|c| c.verdict.fits).count();
            m.next_seed = from + result.checked;
            m.workers = workers;
            m.fits >= FITS
        };
        chunks += 1;
        match result.end {
            _ if full => return me.finish(ctx, SearchState::Full),
            SearchEnd::Failed(words) => return me.fail(ctx, failed_sentence(&words)),
            SearchEnd::Stopped => return me.finish(ctx, SearchState::Stopped),
            _ if me.stop.load(Ordering::SeqCst) => return me.finish(ctx, SearchState::Stopped),
            SearchEnd::Full | SearchEnd::OutOfSeeds => {}
        }
        if chunks.is_multiple_of(SAVE_EVERY) {
            let cached = lock(&me.memory).cached();
            _ = save_shortlist(&ctx.config.paths.shortlists(), APP_VERSION, &key, &cached);
        }
        me.send(ctx);
        #[cfg(test)]
        if me.r#gen == PANICS {
            panic!("on purpose");
        }
    }
}

/// The candidates of a cached shortlist (one [`sound`] says holds together), fought again on the engine's workers;
/// None when a fight cannot be fought, or when the search was silenced or the engine is ending meanwhile. A
/// `StopSearch` does not cut it short (at most `FITS + OTHERS` fights, a fraction of a second on the workers): the
/// stopped list then still holds what was cached.
fn refight(
    ctx: &Ctx,
    me: &Worker,
    setup_for: &(dyn Fn(u64) -> Setup<'static> + Sync),
    sheet: &BeatSheet,
    cached: &CachedShortlist,
) -> Option<Vec<Candidate>> {
    let seeds: Vec<u64> = cached.fit_seeds.iter().chain(&cached.other_seeds).copied().collect();
    let gone = || me.silent.load(Ordering::SeqCst) || ctx.stopping();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out: Mutex<Vec<Option<Candidate>>> = Mutex::new(vec![None; seeds.len()]);
    let failed = AtomicBool::new(false);
    std::thread::scope(|s| {
        for _ in 0..ctx.workers().min(seeds.len()).max(1) {
            s.spawn(|| {
                loop {
                    if gone() || failed.load(Ordering::SeqCst) {
                        return;
                    }
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some(&seed) = seeds.get(i) else {
                        return;
                    };
                    match bf_story::fight(&setup_for(seed), sheet) {
                        Ok(c) => lock(&out)[i] = Some(c),
                        Err(_) => failed.store(true, Ordering::SeqCst),
                    }
                }
            });
        }
    });
    if failed.into_inner() || gone() {
        return None;
    }
    out.into_inner()
        .unwrap_or_else(|e| e.into_inner())
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use bf_story::{Story, Verdict};

    use super::*;
    use crate::engine::tests::starting_draft;
    use crate::engine::{EngineConfig, convert, wait_for};
    use crate::model::draft::Draft;
    use crate::model::shortlist::{Finish, SearchState, Shortlist};
    use crate::model::story::AbilityMark;
    use crate::seam::{EngineHandle, Event, Waker};
    use crate::store::Paths;
    use crate::store::shortlist_cache::{CachedShortlist, cache_file, load_shortlist, save_shortlist};

    /// An engine of its own for a test: `workers` threads, storage under `root`, at most `limit` seeds a search.
    /// The waker is returned so the test can wait until every engine thread has ended.
    fn engine_at(root: &Path, workers: usize, limit: Option<u64>) -> (EngineHandle, Waker) {
        let waker: Waker = Arc::new(|| {});
        let config = EngineConfig {
            workers,
            paths: Paths::under(root),
            clip: None,
            search_limit: limit,
        };
        (crate::engine::spawn(waker.clone(), config), waker)
    }

    /// A new empty folder under the system's temp folder.
    fn temp_root(name: &str) -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("bf-app-search-{}-{name}-{n}", std::process::id()));
        _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the test's temp folder");
        root
    }

    /// Shuts the engine down and waits (5 s at most) until every thread it started has ended.
    fn close(engine: EngineHandle, waker: Waker) {
        engine.send(Command::Shutdown);
        drop(engine);
        let deadline = Instant::now() + Duration::from_secs(5);
        while Arc::strong_count(&waker) > 1 {
            assert!(Instant::now() < deadline, "an engine thread is still running");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// The starting draft under the kit's handicap for this pair, 2.0 and 0.4.
    fn draft() -> Draft {
        let mut d = starting_draft();
        d.handicap = [2.0, 0.4];
        d
    }

    fn start(engine: &EngineHandle, r#gen: u64, draft: &Draft, filters: FilterSpec) {
        engine.send(Command::StartSearch {
            r#gen,
            draft: draft.clone(),
            filters,
        });
    }

    fn list_of(e: Event) -> Shortlist {
        match e {
            Event::Shortlist { list, .. } => list,
            other => panic!("not a shortlist: {other:?}"),
        }
    }

    fn is_list(e: &Event, want: impl Fn(&Shortlist) -> bool) -> bool {
        matches!(e, Event::Shortlist { list, .. } if want(list))
    }

    fn seeds(list: &Shortlist) -> Vec<u64> {
        list.rows.iter().map(|r| r.seed).collect()
    }

    fn sentence_case(name: &str) -> bool {
        let mut chars = name.chars();
        chars.next().is_some_and(char::is_uppercase) && chars.all(|c| !c.is_uppercase())
    }

    #[test]
    fn story_view_maps_a_candidate() {
        let c = Candidate {
            seed: 18_433,
            story: Story {
                seed: 18_433,
                winner: Some(1),
                t_ko: Some(50.5),
                lead_changes: 3,
                lc_t: vec![20.0, 30.0, 41.0],
                winner_hp: 12.5,
                firsts: vec![(14.25, 0, "FLUTTER TONGUE".to_string()), (30.5, 1, "SLIDE".to_string())],
                caps: vec![],
                max_gap: 3.0,
                lows: vec![],
                lead: vec![(9.5, -10.0), (12.0, 4.5)],
            },
            verdict: Verdict {
                fits: false,
                score: 7.25,
                misses: vec!["no K.O.".to_string(), "winner HP 31.00 (needs 30 or less)".to_string()],
            },
        };
        let v = story_view(&c, 8.75);
        assert_eq!(
            v,
            StoryView {
                seed: 18_433,
                winner: Some(1),
                t_ko: Some(50.5),
                winner_hp: 12.5,
                lead_changes: 3,
                score: 7.25,
                fits: false,
                misses: vec!["No K.O.".to_string(), "Winner HP 31.00 (needs 30 or less)".to_string()],
                t_fight: 8.75,
                lead: vec![(9.5, -10.0), (12.0, 4.5)],
                firsts: vec![
                    AbilityMark {
                        t: 14.25,
                        who: 0,
                        name: "Flutter tongue".to_string(),
                    },
                    AbilityMark {
                        t: 30.5,
                        who: 1,
                        name: "Slide".to_string(),
                    },
                ],
            }
        );
    }

    #[test]
    fn filters_map_the_spec() {
        let spec = FilterSpec {
            winner: Some(1),
            ko: (45.0, 55.0),
            min_lead_changes: 3,
            finish: Finish::Close,
            ability: Some("FLUTTER TONGUE".to_string()),
            show_outside: true,
        };
        assert_eq!(
            filters(&spec),
            bf_story::Filters {
                winner: Some(1),
                ko: Some((45.0, 55.0)),
                min_lead_changes: Some(3),
                close_finish: true,
                comeback: false,
                ability: Some("FLUTTER TONGUE".to_string()),
                show_misfits: true,
            }
        );
        let comeback = filters(&FilterSpec {
            finish: Finish::Comeback,
            ..FilterSpec::default()
        });
        assert!(comeback.comeback && !comeback.close_finish && !comeback.show_misfits);
        let any = filters(&FilterSpec::default());
        assert!(!any.comeback && !any.close_finish);
        assert_eq!(
            (any.winner, any.ko, any.min_lead_changes),
            (None, Some((49.0, 53.0)), Some(2))
        );
    }

    #[test]
    fn the_preview_halves_the_workers() {
        assert_eq!(chunk_workers(8, false), 8);
        assert_eq!(chunk_workers(8, true), 4);
        assert_eq!(chunk_workers(1, true), 1);
        assert_eq!(chunk_workers(0, false), 1);
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn the_shortlist_fills_ranked() {
        let root = temp_root("fills");
        let (engine, waker) = engine_at(&root, 4, None);
        let d = draft();
        let t_fight = convert::fight_file(&d).expect("a fight file").grid().t_fight;
        start(&engine, 1, &d, FilterSpec::default());
        let list = list_of(wait_for(&engine, 120.0, |e| is_list(e, |l| l.rows.len() >= 3)));
        assert_eq!(list.state, SearchState::Searching);
        assert!(
            list.checked > 0 && list.checked.is_multiple_of(CHUNK),
            "{}",
            list.checked
        );
        assert!(list.fits_total as usize >= list.rows.len());
        for pair in list.rows.windows(2) {
            assert!(pair[0].score >= pair[1].score, "scores rise down the list");
            if pair[0].score == pair[1].score {
                assert!(pair[0].seed < pair[1].seed);
            }
        }
        for row in &list.rows {
            assert!(row.fits && row.misses.is_empty(), "{row:?}");
            let t_ko = row.t_ko.expect("a K.O.");
            assert!((49.0..=53.0).contains(&t_ko), "{t_ko}");
            assert_eq!(row.t_fight, t_fight);
            assert!(
                row.lead.first().is_some_and(|&(t, _)| f64::from(t) >= t_fight),
                "{:?}",
                row.lead
            );
            assert!(row.firsts.iter().all(|m| sentence_case(&m.name)), "{:?}", row.firsts);
            assert!(row.seed >= 1 && row.seed <= list.checked);
        }
        close(engine, waker);
        std::fs::remove_dir_all(&root).expect("the temp folder");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn filters_are_the_engines() {
        let root = temp_root("filters");
        let (engine, waker) = engine_at(&root, 4, Some(2 * CHUNK));
        let all = FilterSpec {
            show_outside: true,
            ..FilterSpec::default()
        };
        start(&engine, 3, &draft(), FilterSpec::default());
        let done = list_of(wait_for(&engine, 120.0, |e| {
            is_list(e, |l| l.state == SearchState::Stopped)
        }));
        assert_eq!(done.checked, 2 * CHUNK, "the search limit stops the search");
        assert!(done.rows.iter().all(|r| r.fits));
        // from memory: the same fights through other filters
        engine.send(Command::SetFilters {
            r#gen: 3,
            filters: all.clone(),
        });
        let every = list_of(wait_for(&engine, 5.0, |e| is_list(e, |_| true)));
        assert_eq!((every.checked, every.state), (done.checked, SearchState::Stopped));
        assert!(every.rows.iter().any(|r| !r.fits), "some row does not fit");
        // the K.O. and lead-change filters still hold for the others; the fits are all there, first
        assert!(every.rows.len() <= done.rows.len() + OTHERS);
        assert_eq!(seeds(&every)[..done.rows.len()], seeds(&done)[..]);
        engine.send(Command::SetFilters {
            r#gen: 3,
            filters: FilterSpec {
                winner: Some(1),
                ..all.clone()
            },
        });
        let ones = list_of(wait_for(&engine, 5.0, |e| is_list(e, |_| true)));
        assert!(!ones.rows.is_empty() && ones.rows.iter().all(|r| r.winner == Some(1)));
        let want: Vec<u64> = every
            .rows
            .iter()
            .filter(|r| r.winner == Some(1))
            .map(|r| r.seed)
            .collect();
        assert_eq!(seeds(&ones), want);
        // another generation has no search: an empty, idle list
        engine.send(Command::SetFilters { r#gen: 4, filters: all });
        let Event::Shortlist { r#gen, list } = wait_for(&engine, 5.0, |e| is_list(e, |_| true)) else {
            unreachable!()
        };
        assert_eq!(
            (r#gen, list.state, list.rows.len(), list.checked),
            (4, SearchState::Idle, 0, 0)
        );
        close(engine, waker);
        std::fs::remove_dir_all(&root).expect("the temp folder");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn filters_change_while_searching() {
        let root = temp_root("filters-live");
        let (engine, waker) = engine_at(&root, 2, Some(40 * CHUNK));
        start(&engine, 1, &draft(), FilterSpec::default());
        wait_for(&engine, 60.0, |e| is_list(e, |l| l.checked > 0));
        engine.send(Command::SetFilters {
            r#gen: 1,
            filters: FilterSpec {
                show_outside: true,
                ..FilterSpec::default()
            },
        });
        // the search sends the list again soon, through the new filters, without waiting for its chunk to end
        let list = list_of(wait_for(&engine, 5.0, |e| {
            is_list(e, |l| l.rows.iter().any(|r| !r.fits))
        }));
        assert_eq!(list.state, SearchState::Searching);
        close(engine, waker);
        std::fs::remove_dir_all(&root).expect("the temp folder");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn stopping_saves_and_starting_again_resumes() {
        let root = temp_root("resume");
        let d = draft();
        let key = search_key(&convert::fight_file(&d).expect("a fight file"));
        let dir = Paths::under(&root).shortlists();
        let (engine, waker) = engine_at(&root, 4, None);
        start(&engine, 1, &d, FilterSpec::default());
        wait_for(&engine, 60.0, |e| is_list(e, |l| l.checked >= CHUNK));
        engine.send(Command::StopSearch);
        // the stop is seen within one fight per worker; the bound leaves room for other builds on the PC
        let stopped = list_of(wait_for(&engine, 5.0, |e| {
            is_list(e, |l| l.state == SearchState::Stopped)
        }));
        close(engine, waker);
        assert!(cache_file(&dir, APP_VERSION, &key).is_file(), "the cache file exists");
        let saved = load_shortlist(&dir, APP_VERSION, &key).expect("the saved search");
        assert_eq!((saved.checked, saved.next_seed), (stopped.checked, stopped.checked + 1));
        assert_eq!(saved.fit_seeds.len() as u32, stopped.fits_total);
        // a new engine on the same folder picks up where the search stopped
        let (engine, waker) = engine_at(&root, 4, None);
        start(&engine, 1, &d, FilterSpec::default());
        let first = list_of(wait_for(&engine, 60.0, |e| is_list(e, |_| true)));
        assert_eq!(first.checked, saved.checked);
        assert_eq!(first.fits_total, stopped.fits_total);
        assert_eq!(seeds(&first), seeds(&stopped));
        assert_eq!(first.rows, stopped.rows, "the fights are fought again, the same");
        let next = list_of(wait_for(&engine, 60.0, |e| is_list(e, |_| true)));
        assert_eq!(next.checked, saved.checked + CHUNK);
        close(engine, waker);
        std::fs::remove_dir_all(&root).expect("the temp folder");
    }

    /// The draft swapped corner for corner (another search key).
    fn swapped(d: &Draft) -> Draft {
        let mut s = d.clone();
        s.fighters.swap(0, 1);
        s.handicap.swap(0, 1);
        s.bass = s.bass.map(|i| 1 - i);
        s
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn a_filter_change_then_a_new_fight_ends_the_old_search() {
        let root = temp_root("silenced-resend");
        let (engine, waker) = engine_at(&root, 4, None);
        let d = draft();
        start(&engine, 1, &d, FilterSpec::default());
        engine.send(Command::SetFilters {
            r#gen: 1,
            filters: FilterSpec {
                show_outside: true,
                ..FilterSpec::default()
            },
        });
        start(&engine, 2, &swapped(&d), FilterSpec::default());
        // the old worker ends at once, so the new one does not wait out STOP_WAIT for it
        wait_for(&engine, STOP_WAIT.as_secs_f64() * 0.8, |e| {
            matches!(e, Event::Shortlist { r#gen: 2, .. })
        });
        close(engine, waker);
        std::fs::remove_dir_all(&root).expect("the temp folder");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn a_panic_ends_the_list_stopped() {
        let root = temp_root("panic");
        let (engine, waker) = engine_at(&root, 4, None);
        start(&engine, PANICS, &draft(), FilterSpec::default());
        let ended = std::cell::Cell::new(None);
        let Event::Problem { text } = wait_for(&engine, 60.0, |e| {
            if let Event::Shortlist { list, .. } = e
                && list.state == SearchState::Stopped
            {
                ended.set(Some(list.checked));
            }
            matches!(e, Event::Problem { .. })
        }) else {
            unreachable!()
        };
        assert_eq!(
            text,
            "Something went wrong while searching seeds, so it stopped. Try again."
        );
        assert_eq!(ended.get(), Some(CHUNK), "the list it had, stopped");
        // the handler answers the filters from then on
        engine.send(Command::SetFilters {
            r#gen: PANICS,
            filters: FilterSpec::default(),
        });
        let list = list_of(wait_for(&engine, 5.0, |e| is_list(e, |_| true)));
        assert_eq!((list.state, list.checked), (SearchState::Stopped, CHUNK));
        close(engine, waker);
        std::fs::remove_dir_all(&root).expect("the temp folder");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn a_stop_at_once_keeps_what_was_found() {
        let root = temp_root("stop-at-once");
        let d = draft();
        let (engine, waker) = engine_at(&root, 4, None);
        start(&engine, 1, &d, FilterSpec::default());
        wait_for(&engine, 60.0, |e| is_list(e, |l| l.checked >= CHUNK));
        engine.send(Command::StopSearch);
        let stopped = list_of(wait_for(&engine, 5.0, |e| {
            is_list(e, |l| l.state == SearchState::Stopped)
        }));
        // "keep searching", then stop at once: the earlier search's memory stays
        start(&engine, 1, &d, FilterSpec::default());
        engine.send(Command::StopSearch);
        let again = list_of(wait_for(&engine, 5.0, |e| {
            is_list(e, |l| l.state == SearchState::Stopped)
        }));
        assert!(
            again.checked >= stopped.checked,
            "{} < {}",
            again.checked,
            stopped.checked
        );
        assert!(again.fits_total >= stopped.fits_total);
        close(engine, waker);
        // from the cache in a new session, stopped at once: the cached fights are still fought again and kept
        let (engine, waker) = engine_at(&root, 4, None);
        start(&engine, 1, &d, FilterSpec::default());
        engine.send(Command::StopSearch);
        let cached = list_of(wait_for(&engine, 10.0, |e| {
            is_list(e, |l| l.state == SearchState::Stopped)
        }));
        assert!(
            cached.checked >= again.checked,
            "{} < {}",
            cached.checked,
            again.checked
        );
        assert!(cached.fits_total >= again.fits_total);
        close(engine, waker);
        std::fs::remove_dir_all(&root).expect("the temp folder");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn an_oversized_or_repeating_cache_is_a_miss() {
        let d = draft();
        let key = search_key(&convert::fight_file(&d).expect("a fight file"));
        let bad = [
            ("oversized", (1..=FITS as u64 + 1).collect(), vec![], 5_000),
            ("too-many-others", vec![], (1..=OTHERS as u64 + 1).collect(), 5_000),
            ("repeating", vec![3, 3], vec![4], 5_000),
            ("repeating-across", vec![3], vec![3], 5_000),
            ("disagreeing", vec![3], vec![4], 4_000),
        ];
        for (name, fit_seeds, other_seeds, checked) in bad {
            let root = temp_root(name);
            let cache = CachedShortlist {
                next_seed: 5_001,
                checked,
                fit_seeds,
                other_seeds,
            };
            save_shortlist(&Paths::under(&root).shortlists(), APP_VERSION, &key, &cache).expect("written");
            let (engine, waker) = engine_at(&root, 4, None);
            start(&engine, 1, &d, FilterSpec::default());
            let first = list_of(wait_for(&engine, 60.0, |e| is_list(e, |_| true)));
            assert_eq!(first.checked, CHUNK, "{name}: a fresh chunk");
            close(engine, waker);
            std::fs::remove_dir_all(&root).expect("the temp folder");
        }
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn a_draft_that_cannot_be_searched_says_why() {
        let root = temp_root("unsearchable");
        let (engine, waker) = engine_at(&root, 4, None);
        let mut d = draft();
        d.handicap[0] = 0.0;
        start(&engine, 1, &d, FilterSpec::default());
        let Event::Problem { text } = wait_for(&engine, 5.0, |e| matches!(e, Event::Problem { .. })) else {
            unreachable!()
        };
        assert_eq!(
            text,
            "This fight cannot be used: The left fighter's handicap must be above 0, found 0.0."
        );
        let list = list_of(wait_for(&engine, 5.0, |e| is_list(e, |_| true)));
        assert_eq!(
            (list.state, list.checked, list.rows.len()),
            (SearchState::Stopped, 0, 0)
        );
        close(engine, waker);
        std::fs::remove_dir_all(&root).expect("the temp folder");
    }

    #[test]
    fn a_set_up_the_engine_refuses_is_a_sentence() {
        let pair = ["FLUTE".to_string(), "PIANO".to_string()];
        assert_eq!(
            unsearchable("fighters[1]", &pair),
            "This fight cannot be searched yet: the engine cannot fight with Piano."
        );
        assert_eq!(
            unsearchable("rules.swat_cobras", &pair),
            "This fight cannot be searched yet: the engine cannot use one of its settings."
        );
        assert_eq!(
            unsearchable("fighters[7]", &pair),
            "This fight cannot be searched yet: the engine cannot use one of its settings."
        );
    }

    #[test]
    fn an_older_search_never_saves_over_a_newer_one() {
        let at = |next_seed: u64| CachedShortlist {
            next_seed,
            checked: next_seed - 1,
            fit_seeds: vec![],
            other_seeds: vec![],
        };
        assert!(fresher(&at(1_025), None));
        assert!(fresher(&at(1_025), Some(&at(513))));
        assert!(!fresher(&at(513), Some(&at(1_025))));
        assert!(!fresher(&at(513), Some(&at(513))));
        // a cache on disk that does not hold together never holds a newer search back
        let unsound = CachedShortlist {
            checked: 7,
            ..at(4_097)
        };
        assert!(!sound(&unsound));
        assert!(!fresher(&at(1_025), Some(&unsound)));
        assert!(fresher_than_disk(&at(1_025), Some(unsound)));
        assert!(!fresher_than_disk(&at(513), Some(at(1_025))));
        assert!(fresher_than_disk(&at(513), None));
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn a_cache_from_another_version_is_ignored() {
        let root = temp_root("version");
        let d = draft();
        let key = search_key(&convert::fight_file(&d).expect("a fight file"));
        let dir = Paths::under(&root).shortlists();
        let elsewhere = CachedShortlist {
            next_seed: 5_001,
            checked: 5_000,
            fit_seeds: vec![17],
            other_seeds: vec![3, 4],
        };
        save_shortlist(&dir, "0.0.0-another", &key, &elsewhere).expect("written");
        let (engine, waker) = engine_at(&root, 4, None);
        start(&engine, 1, &d, FilterSpec::default());
        let first = list_of(wait_for(&engine, 60.0, |e| is_list(e, |_| true)));
        assert_eq!(first.checked, CHUNK, "a fresh chunk, not the other version's 5,000");
        close(engine, waker);
        std::fs::remove_dir_all(&root).expect("the temp folder");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn a_new_fight_silences_the_old_search() {
        let root = temp_root("silence");
        let (engine, waker) = engine_at(&root, 4, None);
        let d = draft();
        start(&engine, 1, &d, FilterSpec::default());
        wait_for(&engine, 60.0, |e| matches!(e, Event::Shortlist { r#gen: 1, .. }));
        let mut swapped = d.clone();
        swapped.fighters.swap(0, 1);
        swapped.handicap.swap(0, 1);
        swapped.bass = swapped.bass.map(|i| 1 - i);
        start(&engine, 2, &swapped, FilterSpec::default());
        wait_for(&engine, 60.0, |e| matches!(e, Event::Shortlist { r#gen: 2, .. }));
        let until = Instant::now() + Duration::from_secs(3);
        while Instant::now() < until {
            while let Some(e) = engine.poll() {
                assert!(
                    !matches!(e, Event::Shortlist { r#gen: 1, .. }),
                    "the old search answered after the new one"
                );
                assert!(!matches!(e, Event::Problem { .. }), "{e:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        close(engine, waker);
        std::fs::remove_dir_all(&root).expect("the temp folder");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn the_shortlist_does_not_depend_on_the_workers() {
        let all = FilterSpec {
            show_outside: true,
            ..FilterSpec::default()
        };
        let last = |workers: usize| {
            let root = temp_root(&format!("workers-{workers}"));
            let (engine, waker) = engine_at(&root, workers, Some(2 * CHUNK));
            start(&engine, 1, &draft(), all.clone());
            let list = list_of(wait_for(&engine, 120.0, |e| {
                is_list(e, |l| l.state == SearchState::Stopped)
            }));
            close(engine, waker);
            std::fs::remove_dir_all(&root).expect("the temp folder");
            list
        };
        let (one, eight) = (last(1), last(8));
        assert_eq!(one.checked, 2 * CHUNK);
        assert_eq!(eight.checked, 2 * CHUNK);
        assert_eq!(one.fits_total, eight.fits_total);
        assert!(!one.rows.is_empty());
        assert_eq!(seeds(&one), seeds(&eight));
    }
}
