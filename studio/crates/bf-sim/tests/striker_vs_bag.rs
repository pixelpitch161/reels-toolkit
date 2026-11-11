//! Striker parity with the Python kit, past what the reference recordings can show: the FLUTE against a bag.
//!
//! In every committed recording the TROMBONE's lance lands before a cobra has lunged, so the span the lance leaves
//! alone holds basket drops only (`striker_prefix.rs`). Here the FLUTE fights a bag, a fighter with no weapon and no
//! abilities, so nothing but the engine core and the striker acts for the whole fight: baskets drop and are taken
//! back, cobras rise, lunge, bite and miss, the reach grows to its cap and the lunge shortens to its floor, and with
//! FLUTTER TONGUE the strike grid doubles and with SNAKE DANCE the cobras leave their baskets.
//!
//! The Python side is `fight.py` itself, unchanged: `Fight('FLUTE', 'BAG', seed, Rules(bpm, t_fight, LV_DMG=(2.0,
//! 5.0), LOW_HP=-1.0)).run()` at the episode's tempo and start time, with `fighters.FIGHTERS['BAG'] =
//! dict(weapon=dict(kind='none'), abilities=<three of kind 'none'>)` added at run time. `LV_DMG` brings the two
//! unlocks early (as the `early_levels` scenario does); `LOW_HP = -1.0` keeps every `low` ability from firing. The
//! vectors below are what that printed on the reference PC (CPython 3.13, numpy 2.5): every event before the end of
//! the window, and every 15th or 30th recorded frame as `[frame, P, V, HP, BL, COB]`.
//!
//! The engine does not fire abilities yet. What the FLUTE's two unlocks do to the striker is two assignments in
//! `trigger` (haste: `fight.py` 2849 to 2851; dance: 2878 to 2879), made at the end of the step that fires them; the
//! test makes them by hand at the times of Python's `ability` events. Once the engine fires abilities itself the
//! replay only repeats what the engine has just done and can be dropped.
//!
//! The windows here end long before the two engines drift apart. When this test was written the same comparison was
//! run over whole fights for seeds 1 to 40 in both corners, with and without the early levels (160 fights): every
//! fight agreed to 1e-9 for at least 13.4 s of fighting (half of them for 22 s), every first difference was a float
//! drifting past the tolerance in an event that was otherwise the same, and the kinds, times and whole numbers of
//! all events were equal for at least 27 s of fighting in every fight. The drift starts from last-bit differences in
//! the arithmetic (the maths library, numpy's fused dot product) and grows with the fight's chaos.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bf_core::event::{Event, Value};
use bf_core::recording::Recording;
use bf_sim::ball::Slot;
use bf_sim::roster::{AbilityCfg, AbilityKind, FighterCfg, WeaponCfg, parse_roster};
use bf_sim::schedule::Schedule;
use bf_sim::{Fight, Rules, Setup};
use serde_json::Value as Json;

/// Positions, velocities, hit points, the reach, the cobra rows and the float fields of events agree to this.
const TOL: f64 = 1e-9;

/// One Python fight's vectors.
struct Vectors {
    /// What to call the fight in messages.
    name: &'static str,
    /// The seed.
    seed: u64,
    /// The FLUTE's corner: 0 left, 1 right. The bag has the other.
    flute: usize,
    /// The window: everything before this time.
    t_end: f64,
    /// Python's events before `t_end`, one JSON list per line.
    events: &'static str,
    /// Python's frames, one `[frame, P, V, HP, BL, COB]` per line.
    frames: &'static str,
}

/// `Fight('FLUTE', 'BAG', seed=9, ...)`: until 13.5 s, every 15th frame.
const LEFT: Vectors = Vectors {
    name: "FLUTE (left) against a bag, seed 9",
    seed: 9,
    flute: 0,
    t_end: 13.5,
    events: r#"
[3.59657, "basket", 0, 0, -1.85, 1.85, -1]
[4.93824, "strike", 0, 0, -1.047030484096805, 1.4794550640588855, 0.12]
[5.06324, "hit", 0, 1, 1.08, -0.7952709658614407, 0.9036610671690423, "fang", 0]
[5.29657, "rope", 0, 0, 1, 2.3992764489057437, 0.5616863464453943, 2.6]
[5.38824, "basket", 0, 1, 2.1960559248897824, 0.48654738808764875, -1]
[6.08824, "rope", 1, 1, -1, 0.004332205488560767, -2.390251259856829, 2.6875973829413207]
[6.7299, "strike", 0, 1, -2.4107232116398207, 1.6628511610763184, 0.11699999999999999]
[6.8549, "hit", 0, 1, 1.08, 1.3426762246785282, -0.8618040509936673, "fang", 1]
[6.8549, "level", 0, 2]
[7.1799, "basket", 0, 2, -2.1731853414533493, -1.1289402166038818, 0]
[7.1799, "ability", 0, "FLUTTER TONGUE", "haste", 1]
[7.27157, "rope", 0, 0, -1, -2.382950029727497, -1.2191069666331764, 2.6]
[7.4549, "clash", -1, -1.3535624713512124, -1.5100810983797381]
[7.6299, "strike", 0, 2, -0.48509775131391925, 1.5495704150211482, 0.056999999999999995]
[7.63824, "rope", 0, 0, -1, -2.3878122166289626, -1.3275220196187667, 2.6]
[7.68824, "hit", 0, 1, 1.08, -0.8569147744554975, -1.8117494679367652, "fang", 2]
[7.81324, "rope", 1, 1, -1, 0.01684244546802951, -2.3854855710371314, 3.5355851617997014]
[8.3049, "strike", 0, 1, -1.9751532027567602, 1.85, 0.055499999999999994]
[8.36324, "hit", 0, 1, 1.08, 1.4709751786614411, -0.8983205638594827, "fang", 1]
[8.6299, "rope", 1, 1, -1, 1.2564226478242209, -2.38558065582248, 3.2444293248508402]
[8.76324, "clash", -1, 0.850831550082907, -1.4920381730037169]
[8.97157, "basket", 0, 3, 0.5560901756669134, -0.4793618122494308, 1]
[9.19657, "rope", 1, 0, 1, 2.3858820508672687, -2.2032885944348206, 2.745058496805263]
[9.42157, "strike", 0, 3, -0.9565953288225757, 1.798844162688828, 0.05399999999999999]
[9.4799, "hit", 0, 1, 1.08, 1.5646926636450627, -1.658678547528016, "fang", 3]
[9.4799, "level", 0, 3]
[9.52157, "rope", 1, 1, -1, 1.6803987346568925, -2.395985649414279, 3.764993073679463]
[9.64657, "strike", 0, 3, -0.8821316573208217, 1.603743705808383, 0.05249999999999999]
[9.7049, "hit", 0, 1, 1.08, 1.283641236874066, -1.6555512547686617, "fang", 3]
[9.87157, "strike", 0, 3, -0.7480261146325924, 2.0600000000000005, 0.05099999999999999]
[9.87157, "ability", 0, "SNAKE DANCE", "dance", 1]
[9.88824, "rope", 1, 0, 1, 2.3990149323420833, -2.179231806769901, 3.4024450692259802]
[9.9299, "hit", 0, 1, 1.08, 1.9868612775040746, -1.7434416755063036, "fang", 3]
[9.9799, "rope", 1, 1, -1, 2.3899574706016993, -2.3770398888733837, 3.7397230759896956]
[9.98824, "rope", 1, 0, 1, 2.396893158979267, -2.3490128825030707, 3.715001212703186]
[10.0799, "rope", 0, 1, 1, 0.8067549639557927, 2.386869529053143, 2.6]
[10.09657, "strike", 0, 3, -0.746111830028952, 1.741371436996325, 0.04949999999999999]
[10.14657, "hit", 0, 1, 1.08, 1.5830658016459838, -1.634229399328063, "fang", 3]
[10.23824, "rope", 1, 0, 1, 2.384252948199295, -2.0616515731103533, 3.6213611457235753]
[10.32157, "strike", 0, 3, -0.8314867287837258, 1.6391821108449331, 0.04799999999999999]
[10.37157, "hit", 0, 1, 1.08, 1.8795936475771016, -1.735629439928701, "fang", 3]
[10.39657, "rope", 1, 1, -1, 2.0500475418511503, -2.392735913940264, 3.8172263547864076]
[10.54657, "strike", 0, 3, -0.6841720678491452, 1.3053590695294837, 0.046499999999999986]
[10.59657, "hit", 0, 1, 1.08, 1.7406996043571916, -1.6664268659512445, "fang", 3]
[10.62157, "rope", 1, 0, 1, 2.3772555667192843, -1.7858617500693534, 3.8172263547864076]
[10.77157, "basket", 0, 4, 0.9858740526529475, 0.6192550924447766, 2]
[10.77157, "strike", 0, 3, -0.8823556401245461, 0.9316960625998006, 0.044999999999999984]
[10.82157, "hit", 0, 1, 1.08, 1.6467799643767487, -1.4699502313010226, "fang", 3]
[10.92157, "rope", 1, 1, -1, 1.8847629613874572, -2.397299899449018, 3.599206684571857]
[11.21324, "strike", 0, 3, -0.35215416433645635, 0.7307565715627874, 0.08699999999999997]
[11.3049, "hit", 0, 1, 1.08, 1.747746178008692, -1.4408235285976165, "fang", 3]
[11.3299, "rope", 1, 0, 1, 2.3918403759252307, -1.2487809780235302, 3.8172263547864076]
[11.36324, "clash", -1, 1.7051842820719356, -1.0961863289481455]
[11.46324, "rope", 1, 0, 1, 2.3978928903002177, -1.5995911365485873, 3.4570128192605774]
[11.66324, "strike", 0, 3, -0.8189322576795088, 0.9399999999999998, 0.08399999999999996]
[11.66324, "strike", 0, 4, -1.23501678410455, 1.0199025005749967, 0.08399999999999996]
[11.7299, "rope", 1, 1, -1, 2.1162078756549896, -2.3923303264622438, 3.024833994234314]
[11.7549, "hit", 0, 1, 1.08, 1.8812156763084724, -1.7598288703568477, "fang", 3]
[11.7549, "hit", 0, 1, 1.08, 2.0665795548084476, -1.7203890290380135, "fang", 4]
[11.7799, "rope", 1, 1, -1, 2.1386821834371386, -2.3936966004881577, 3.8172263547864076]
[12.02157, "rope", 1, 0, 1, 2.3930501574555842, -1.593868960019014, 3.2444293248508402]
[12.11324, "strike", 0, 3, -0.3970208760224273, 1.310507289730893, 0.07799999999999996]
[12.11324, "strike", 0, 4, -0.8943143489222798, 1.0988384057777711, 0.07799999999999996]
[12.19657, "hit", 0, 1, 1.08, 1.6898891974146477, -1.3532037673626363, "fang", 3]
[12.19657, "hit", 0, 1, 1.08, 1.625917409841645, -1.012763678989672, "fang", 4]
[12.26324, "rope", 1, 0, 1, 2.3942977035779944, -1.2498425535184308, 3.69081559417405]
[12.56324, "basket", 0, 5, -1.807085606033551, -1.9180655831668576, 3]
[12.56324, "strike", 0, 4, -1.3142268619948643, 2.158968661425052, 0.07199999999999995]
[12.63824, "hit", 0, 1, 1.08, 1.5915765488943456, -1.4875911297748634, "fang", 4]
[12.7299, "rope", 1, 1, -1, 1.502073420916347, -2.380176070499305, 3.6213611457235753]
[12.79657, "rope", 0, 0, -1, -2.3974770441036433, -2.1187074037473006, 2.6]
[13.01324, "strike", 0, 4, -1.3209646232330599, 1.7937299578016104, 0.07]
[13.08824, "hit", 0, 1, 1.08, 1.1031546380281294, -0.7855754279656937, "fang", 4]
[13.09657, "rope", 0, 1, -1, -1.6707388730903925, -2.3980054084170113, 2.6]
[13.46324, "strike", 0, 4, -1.2653316026383725, 2.6399999999999997, 0.07]
[13.46324, "rope", 1, 1, -1, 1.946304404817355, -2.3761731752852526, 3.084577529524024]
"#,
    frames: r#"
[0, [-1.85, 1.85, 1.85, -1.85], [2.4882284201536926, -0.7541348215932351, -2.072054415237249, 1.5705382836135584], [100.0, 100.0], [1.4, 0.0], [[], []]]
[15, [-0.6058857899236343, 1.47293258920353, 0.8139727923817704, -1.0647308581935184], [2.488228420152735, -0.7541348215929449, -2.0720544152364524, 1.570538283612954], [100.0, 100.0], [1.4, 0.0], [[[0, -1.85, 1.85, -1.85, 1.85, 3.59656862745098]], []]]
[30, [0.6382284201527335, 1.09586517840706, -0.22205441523645536, -0.27946171638704204], [2.488228420152735, -0.7541348215929449, -2.0720544152364524, 1.570538283612954], [100.0, 100.0], [1.4, 0.0], [[[0, -1.85, 1.85, -1.85, 1.85, 3.59656862745098]], []]]
[45, [1.8823426302290993, 0.7187977676105901, -1.1689959517118298, 0.3779602191748597], [2.488228420152735, -0.7541348215929449, 1.4808253788227874, -3.518291238812201], [100.0, 98.92], [1.47, 0.0], [[[0, -1.85, 1.85, -1.85, 1.85, 3.59656862745098]], []]]
[60, [1.6880046148498828, 0.2986999921932849, -0.5348179533900657, -1.1287825252780204], [-2.4386462881915176, -0.9016675002929484, 1.135297875507908, -2.697352858725947], [100.0, 98.92], [1.47, 0.0], [[[0, -1.85, 1.85, -1.85, 1.85, 3.59656862745098], [1, 2.1960559248897824, 0.48654738808764875, 2.1960559248897824, 0.48654738808764875, 5.388235294117647]], []]]
[75, [0.4686814707541258, -0.15213375795318934, 0.004332205488560767, -2.390251259856829], [-2.4386462881915176, -0.9016675002929484, 1.1010121177954437, 2.451724293075234], [100.0, 98.92], [1.47, 0.0], [[[0, -1.85, 1.85, -1.85, 1.85, 3.59656862745098], [1, 2.1960559248897824, 0.48654738808764875, 2.1960559248897824, 0.48654738808764875, 5.388235294117647]], []]]
[90, [-0.750641673341633, -0.6029675080996629, 0.5467648341294593, -1.1823669582213534], [-2.4386462881915176, -0.9016675002929484, 1.0747533489204681, 2.3932514928975817], [100.0, 98.92], [1.47, 0.0], [[[0, -1.85, 1.85, -1.85, 1.85, 3.59656862745098], [1, 2.1960559248897824, 0.48654738808764875, 2.1960559248897824, 0.48654738808764875, 5.388235294117647]], []]]
[105, [-1.9699648174373885, -1.0538012582461362, 0.14694808145520563, -1.0150440993010745], [-2.4386462881915176, -0.9016675002929484, -2.726267711895387, -1.8656510054832502], [100.0, 97.84], [1.54, 0.0], [[[0, -1.85, 1.85, -1.85, 1.85, 3.59656862745098], [1, 2.1960559248897824, 0.48654738808764875, 2.1960559248897824, 0.48654738808764875, 5.388235294117647]], []]]
[120, [-2.282781250267368, -1.33992964590569, -0.5402955793431772, -1.9458694138109027], [-2.588130662073385, 0.24815252573846747, 1.6972645133064561, -2.2127549714610963], [100.0, 97.84], [1.54, 0.0], [[[1, 2.1960559248897824, 0.48654738808764875, 2.1960559248897824, 0.48654738808764875, 5.388235294117647], [2, -2.1731853414533493, -1.1289402166038818, -2.1731853414533493, -1.1289402166038818, 7.1799019607843135]], []]]
[135, [-1.2247210931383843, -1.2005612443519236, 0.7810622104932157, -1.9175369260242683], [2.584646941090187, 0.28213505614853845, 2.6042825340347617, 1.5946597285298834], [100.0, 96.76], [1.61, 0.0], [[[1, 2.1960559248897824, 0.48654738808764875, 2.1960559248897824, 0.48654738808764875, 5.388235294117647], [2, -2.1731853414533493, -1.1289402166038818, -2.1731853414533493, -1.1289402166038818, 7.1799019607843135]], []]]
[150, [0.06760237740670826, -1.0594937162776534, 1.2868041691964691, -2.2814439588629094], [2.584646941090187, 0.28213505614853845, -0.7392828010278524, -3.2357304994259035], [100.0, 95.68], [1.6800000000000002, 0.0], [[[1, 2.1960559248897824, 0.48654738808764875, 2.1960559248897824, 0.48654738808764875, 5.388235294117647], [2, -2.1731853414533493, -1.1289402166038818, -2.1731853414533493, -1.1289402166038818, 7.1799019607843135]], []]]
[165, [0.5841496668932805, -0.1555753626379066, 2.1190546114021074, -2.1510512050285318], [0.22447592981093686, 2.5902915968921945, 2.7501446266630354, -0.48687978957848166], [100.0, 95.68], [1.6800000000000002, 0.0], [[[2, -2.1731853414533493, -1.1289402166038818, -2.1731853414533493, -1.1289402166038818, 7.1799019607843135], [3, 0.5560901756669134, -0.4793618122494308, 0.5560901756669134, -0.4793618122494308, 8.97156862745098]], []]]
[180, [0.6963876317987489, 1.1395704358081917, 1.752336026913287, -2.1630508993442783], [0.22447592981093686, 2.5902915968921945, 1.0556544745349554, 3.41823556980238], [100.0, 94.60000000000001], [1.7500000000000002, 0.0], [[[2, -2.1731853414533493, -1.1289402166038818, -2.1731853414533493, -1.1289402166038818, 7.1799019607843135], [3, 0.5560901756669134, -0.4793618122494308, 0.5560901756669134, -0.4793618122494308, 8.97156862745098]], []]]
[195, [0.8089393430862458, 2.365313255435968, 2.2314404847310323, -2.032707469172811], [0.262125495654379, -2.5867528340610253, -1.6023215442864633, 3.0632504470318014], [100.0, 92.44000000000001], [1.8900000000000003, 0.0], [[[2, -2.1731853414533493, -1.1289402166038818, -1.7102234347387455, -1.2419991023329613, 7.1799019607843135], [3, 0.5560901756669134, -0.4793618122494308, 0.6883327089589711, -0.60632031675738, 8.97156862745098]], []]]
[210, [0.9400020909134322, 1.0719368384054553, 2.3242827421710794, -1.7709699759987425], [0.262125495654379, -2.5867528340610253, 1.3458607893436834, 3.051433204230341], [100.0, 90.28000000000002], [2.0300000000000002, 0.0], [[[2, -2.1731853414533493, -1.1289402166038818, -0.636504832007571, -1.4734817592132656, 7.1799019607843135], [3, 0.5560901756669134, -0.4793618122494308, 0.9655852024038925, -0.8729020181085266, 8.97156862745098]], []]]
[225, [1.0710648387406223, -0.22143957862505645, 2.0810330739350054, -1.86569891030963], [0.262125495654379, -2.5867528340610253, 1.123719070441396, 3.04361250731297], [100.0, 88.12000000000002], [2.17, 0.0], [[[3, 0.5560901756669134, -0.4793618122494308, 1.1844519356503878, -1.1448488894685434, 8.97156862745098], [4, 0.9858740526529475, 0.6192550924447766, 1.2157929499644957, -0.05672633285397352, 10.77156862745098]], []]]
[240, [0.5746637934176596, -1.1154983008450696, 2.2611947828466925, -1.9917412366146376], [-2.4638786890874615, -0.8302420161949479, -1.0588365689550743, -3.0375173014775885], [100.0, 87.04000000000002], [2.2399999999999998, 0.0], [[[3, 0.5560901756669134, -0.4793618122494308, 1.2431037596174954, -1.1833535397635995, 8.97156862745098], [4, 0.9858740526529475, 0.6192550924447766, 1.683005700673712, -0.751566208840351, 10.77156862745098]], []]]
[255, [-0.6572755511260711, -1.5306193089425462, 2.326528977729948, -1.392252233388288], [-2.4638786890874615, -0.8302420161949479, -0.9840704454550306, 2.982584836964642], [100.0, 84.88000000000002], [2.3799999999999994, 0.0], [[[3, 0.5560901756669134, -0.4793618122494308, 0.8107496171297679, -0.7387422548206445, 8.97156862745098], [4, 0.9858740526529475, 0.6192550924447766, 1.4230824834934663, -0.2577499229137532, 10.77156862745098]], []]]
[270, [-1.8892148956697994, -1.9457403170400227, 1.5936363266856863, -1.9756435659170293], [-2.4638786890874615, -0.8302420161949479, -2.2699391005579943, -2.057703974260462], [100.0, 82.72000000000003], [2.4, 0.0], [[[4, 0.9858740526529475, 0.6192550924447766, 1.0304644702418868, 0.5298102942572145, 10.77156862745098], [5, -1.807085606033551, -1.9180655831668576, -1.807085606033551, -1.9180655831668576, 12.563235294117646]], []]]
[285, [-1.6909260445074272, -2.3941255030319972, 1.5228857180890407, -1.2143245625781158], [2.4224605700441644, -0.9442906261189413, 0.8590790093519154, -2.453972953328693], [100.0, 80.56000000000003], [2.4, 0.0], [[[4, 0.9858740526529475, 0.6192550924447766, 0.9858740526529475, 0.6192550924447766, 10.77156862745098], [5, -1.807085606033551, -1.9180655831668576, -1.807085606033551, -1.9180655831668576, 12.563235294117646]], []]]
"#,
};

/// `Fight('BAG', 'FLUTE', seed=1, ...)`: until 13.2 s, every 30th frame.
const RIGHT: Vectors = Vectors {
    name: "a bag against FLUTE (right), seed 1",
    seed: 1,
    flute: 1,
    t_end: 13.2,
    events: r#"
[3.59657, "basket", 1, 0, 1.85, -1.85, -1]
[4.56324, "clash", -1, 0.5194451766991839, 0.32366947943381696]
[4.93824, "strike", 1, 0, 2.4788935733252497, 1.4341848846257172, 0.12]
[5.06324, "hit", 1, 0, 1.08, 1.0949403301052558, -0.7016625043321821, "fang", 0]
[5.27157, "rope", 1, 1, 1, 0.9923937031869409, 2.3977865561537324, 2.6]
[5.38824, "basket", 1, 1, 0.9938730824814046, 2.116123774537216, -1]
[6.23824, "rope", 0, 0, -1, -2.3810623551063856, 0.7555853450774286, 2.659028140055343]
[7.12157, "rope", 1, 1, -1, 1.017656949600108, -2.3878529008563016, 2.6]
[7.1799, "basket", 1, 2, 1.021993863055731, -2.2579252626045974, 0]
[7.1799, "strike", 1, 1, -3.0969611948212736, 0.9446411791277417, 0.11699999999999999]
[7.3049, "hit", 1, 0, 1.08, 0.2021192813982287, 1.6835771564986337, "fang", 1]
[7.3049, "level", 1, 2]
[7.6299, "strike", 1, 1, 3.0235311726289553, 1.7568306630922819, 0.11399999999999999]
[7.6299, "ability", 1, "FLUTTER TONGUE", "haste", 1]
[7.7299, "rope", 0, 1, 1, -1.4404720095607315, 2.397220113710186, 3.024833994234314]
[8.07157, "rope", 0, 0, -1, -2.3864304834846357, 2.2578221200540565, 2.772876191772477]
[8.5299, "strike", 1, 1, -3.1380898570330036, 1.78, 0.056999999999999995]
[8.58824, "hit", 1, 0, 1.08, -0.40864724103301286, 2.14961743505106, "fang", 1]
[8.97157, "basket", 1, 3, 1.1773999285488899, 2.3978151080814336, 1]
[8.97157, "rope", 1, 1, 1, 1.1781227474581604, 2.380530285543282, 2.6]
[9.0049, "rope", 0, 0, -1, -2.3873827670596603, 2.022759850566015, 3.034253435043226]
[9.42157, "strike", 1, 3, -2.965496486861192, 1.85, 0.055499999999999994]
[9.4799, "hit", 1, 0, 1.08, -0.45957086291254134, 2.12549166694754, "fang", 3]
[9.89657, "rope", 0, 0, -1, -2.397194817477228, 1.617301336308545, 3.034253435043226]
[10.54657, "strike", 1, 3, -2.5682690064106426, 1.8021676528543185, 0.05399999999999999]
[10.6049, "hit", 1, 0, 1.08, -0.17635488743579436, 1.7121253986327047, "fang", 3]
[10.6049, "level", 1, 3]
[10.77157, "basket", 1, 4, 1.405209403107496, -2.2722646911619715, 2]
[10.77157, "ability", 1, "SNAKE DANCE", "dance", 1]
[10.81324, "rope", 1, 1, -1, 1.411546705125617, -2.3978898676276494, 2.6]
[11.49657, "rope", 0, 0, -1, -2.384385879480313, -0.7668045525195489, 2.7244156921928995]
[11.66324, "strike", 1, 4, 2.99670527411333, 1.5023874196957188, 0.10499999999999998]
[11.77157, "hit", 1, 0, 1.08, -1.4888146358552259, -0.8280500990558979, "fang", 4]
[11.92157, "rope", 0, 0, -1, -2.384532930999499, -1.3171106131720842, 3.476014550838232]
[12.11324, "strike", 1, 3, -2.0728689082025675, 1.5496271971721685, 0.10199999999999998]
[12.11324, "strike", 1, 4, -3.1278668695410037, 0.8697173195042162, 0.10199999999999998]
[12.22157, "hit", 1, 0, 1.08, -1.7116690907115457, -0.8532085229118613, "fang", 3]
[12.22157, "hit", 1, 0, 1.08, -1.3504686097624068, -0.7862411937503662, "fang", 4]
[12.51324, "rope", 0, 0, -1, -2.381216572061503, -1.6200089307433168, 3.2033971984658876]
[12.56324, "basket", 1, 5, 1.8064965968783246, 2.1131873164567128, 3]
[12.56324, "strike", 1, 4, -2.9541526943344993, 0.9811792500092985, 0.09599999999999997]
[12.66324, "hit", 1, 0, 1.08, -1.8024455637400292, -1.12741606097241, "fang", 4]
[12.67157, "rope", 1, 1, 1, 1.8329525704885539, 2.3846352645137103, 2.6]
[12.8049, "rope", 0, 0, -1, -2.3776421094370987, -1.8736379147163622, 3.4954375897695638]
[13.01324, "strike", 1, 4, -2.8403379883210067, 1.1606108604746361, 0.09299999999999997]
[13.11324, "hit", 1, 0, 1.08, -1.3566725347434878, -1.59661481513665, "fang", 4]
"#,
    frames: r#"
[0, [-1.85, 1.85, 1.85, -1.85], [1.8670792192515113, -1.8094239937171077, -0.8015506516630289, 2.4733613874277602], [100.0, 100.0], [0.0, 1.4], [[], []]]
[30, [0.0007095332170375286, 0.028477450124372913, 1.0672616947610822, 0.627124931044817], [1.2727978976888197, -2.2671536144764697, -0.10956291449875445, 2.5976905065387097], [100.0, 100.0], [0.0, 1.4], [[], [[0, 1.85, -1.85, 1.85, -1.85, 3.59656862745098]]]]
[60, [-0.881203054033773, -0.16170127675051305, 0.9967180426630656, 1.5744645791208387], [-2.513336519008914, 1.4992508972236653, 0.013655808871978429, -2.5999641379986094], [98.92, 100.0], [0.0, 1.47], [[], [[0, 1.85, -1.85, 1.85, -1.85, 3.59656862745098], [1, 0.9938730824814046, 2.116123774537216, 0.9938730824814046, 2.116123774537216, 5.388235294117647]]]]
[90, [-1.599348998435544, 1.2470179376249424, 1.0103738515350487, -1.0254995588777722], [2.2210607139091723, 1.3962939426931362, 0.013655808871978429, -2.5999641379986094], [98.92, 100.0], [0.0, 1.47], [[], [[0, 1.85, -1.85, 1.85, -1.85, 3.59656862745098], [1, 0.9938730824814046, 2.116123774537216, 0.9938730824814046, 2.116123774537216, 5.388235294117647]]]]
[120, [-1.0027503535927862, 2.353612622841425, 1.0581348085192563, -1.1751949438404037], [-3.196672757145049, 0.3590675706032431, 0.08673826911247026, 2.5985527650340625], [97.84, 100.0], [0.0, 1.54], [[], [[1, 0.9938730824814046, 2.116123774537216, 0.9938730824814046, 2.116123774537216, 5.388235294117647], [2, 1.021993863055731, -2.2579252626045974, 1.021993863055731, -2.2579252626045974, 7.1799019607843135]]]]
[150, [-1.005351323719465, 2.0868142686872555, 1.144873077631717, 1.4233578211936573], [-2.5973108500843214, -0.11822160552034121, 0.08673826911247026, 2.5985527650340625], [96.76, 100.0], [0.0, 1.61], [[], [[1, 0.9938730824814046, 2.116123774537216, 0.9938730824814046, 2.116123774537216, 5.388235294117647], [2, 1.021993863055731, -2.2579252626045974, 1.021993863055731, -2.2579252626045974, 7.1799019607843135]]]]
[180, [-1.4277622402244987, 1.8643649387209844, 1.2562828056816526, 0.7791031772819413], [-3.467934911640627, -0.8787310333042712, 0.12674604036243092, -2.5969088242075893], [95.68, 100.0], [0.0, 1.6800000000000002], [[], [[2, 1.021993863055731, -2.2579252626045974, 1.021993863055731, -2.2579252626045974, 7.1799019607843135], [3, 1.1773999285488899, 2.3978151080814336, 1.1773999285488899, 2.3978151080814336, 8.97156862745098]]]]
[210, [-0.508534401610582, 1.195875422530826, 1.3830288460440725, -1.8178056469256454], [2.6062543992194565, -0.5815461225856546, 0.12674604036243092, -2.5969088242075893], [95.68, 100.0], [0.0, 1.6800000000000002], [[], [[2, 1.021993863055731, -2.2579252626045974, 1.021993863055731, -2.2579252626045974, 7.1799019607843135], [3, 1.1773999285488899, 2.3978151080814336, 1.1773999285488899, 2.3978151080814336, 8.97156862745098]]]]
[240, [-2.2103901337151592, -0.9440225499462755, 1.5872899583935682, -0.3905684412169035], [1.8900154135438458, -1.9250168745273215, 0.22676548808768876, 2.5900921631106395], [94.60000000000001, 100.0], [0.0, 1.7500000000000002], [[], [[3, 1.1773999285488899, 2.3978151080814336, -0.17931829498435245, 1.2211490793179995, 8.97156862745098], [4, 1.405209403107496, -2.2722646911619715, -0.09736666677217462, -1.3654153363851516, 10.77156862745098]]]]
[270, [-2.1505196212039643, -1.6679226813197952, 1.8140554464812473, 2.1995237218937347], [3.0306471775589565, -0.6294390645871082, 0.22676548808768876, 2.5900921631106395], [91.36000000000001, 100.0], [0.0, 1.9600000000000004], [[], [[4, 1.405209403107496, -2.2722646911619715, -0.934398997314629, -1.3969069367840496, 10.77156862745098], [5, 1.8064965968783246, 2.1131873164567128, 1.8064965968783246, 2.1131873164567128, 12.563235294117646]]]]
"#,
};

/// A JSON value as a recording value: a number written without a point or an exponent is a whole number.
fn value(j: &Json) -> Value {
    match j {
        Json::Number(n) if n.is_i64() => Value::Int(n.as_i64().expect("a whole number")),
        Json::Number(n) => Value::Float(n.as_f64().expect("a float")),
        Json::String(s) => Value::Str(s.clone()),
        Json::Array(a) => Value::List(a.iter().map(value).collect()),
        other => panic!("a vector holds {other}"),
    }
}

fn lines(text: &str) -> Vec<Vec<Json>> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{l}: {e}")))
        .collect()
}

fn number(j: &Json) -> f64 {
    j.as_f64().unwrap_or_else(|| panic!("{j} is not a number"))
}

fn numbers<const N: usize>(j: &Json) -> [f64; N] {
    let list = j.as_array().filter(|l| l.len() == N);
    let list = list.unwrap_or_else(|| panic!("{j} is not a list of {N}"));
    std::array::from_fn(|i| number(&list[i]))
}

/// Python's events: `[t, kind, who, ...]` per line.
fn events(text: &str) -> Vec<Event> {
    let event = |row: Vec<Json>| Event {
        t: number(&row[0]),
        kind: row[1].as_str().expect("a kind").to_string(),
        fields: row[2..].iter().map(value).collect(),
    };
    lines(text).into_iter().map(event).collect()
}

/// What a frame holds of the fight.
#[derive(Debug)]
struct Frame {
    /// Both balls' positions: x, y of the left one, then of the right one.
    p: [f64; 4],
    /// Their velocities.
    v: [f64; 4],
    /// Their hit points.
    hp: [f64; 2],
    /// Their reach (`blade_len`).
    bl: [f64; 2],
    /// The `COB` row: for each fighter its baskets as `[id, home x, home y, cobra x, cobra y, t_drop]`.
    cob: Value,
}

/// Python's frames: `[frame, P, V, HP, BL, COB]` per line.
fn frames(text: &str) -> Vec<(usize, Frame)> {
    let frame = |row: Vec<Json>| {
        let i = row[0].as_u64().expect("a frame number") as usize;
        let f = Frame {
            p: numbers(&row[1]),
            v: numbers(&row[2]),
            hp: numbers(&row[3]),
            bl: numbers(&row[4]),
            cob: value(&row[5]),
        };
        (i, f)
    };
    lines(text).into_iter().map(frame).collect()
}

/// The state of the fight as a frame records it (`Fight.snap`).
fn snap(fight: &Fight) -> Frame {
    let [a, b] = &fight.balls;
    let sides = fight.striker.iter().map(|st| {
        let baskets = st.iter().flat_map(|st| &st.baskets).map(|bk| {
            Value::List(vec![
                Value::Int(bk.id),
                Value::Float(bk.home[0]),
                Value::Float(bk.home[1]),
                Value::Float(bk.cp[0]),
                Value::Float(bk.cp[1]),
                Value::Float(bk.t_drop),
            ])
        });
        Value::List(baskets.collect())
    });
    Frame {
        p: [a.p[0], a.p[1], b.p[0], b.p[1]],
        v: [a.v[0], a.v[1], b.v[0], b.v[1]],
        hp: [a.hp.max(0.0), b.hp.max(0.0)],
        bl: [a.blade_len, b.blade_len],
        cob: Value::List(sides.collect()),
    }
}

/// A fighter with no weapon and no abilities: Python's `dict(weapon=dict(kind='none'), abilities=...)`.
fn bag() -> FighterCfg {
    let nothing = || AbilityCfg {
        name: "NOTHING".into(),
        kind: AbilityKind::Unsupported("none".into()),
        params: BTreeMap::new(),
        slow: None,
    };
    FighterCfg {
        name: "BAG".into(),
        color: [255, 255, 255],
        dark: [0, 0, 0],
        weapon: WeaponCfg::Unsupported("none".into()),
        unlock1: nothing(),
        unlock2: nothing(),
        low: nothing(),
        register: (60, 72),
        idiom: String::new(),
        pp: String::new(),
        raw: Json::Null,
    }
}

/// The episode's tempo and start time: `bpm` and `t_fight` of the reference recordings' rules.
fn tempo() -> (f64, f64) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/fights/flute_vs_trombone/ep392.bfrec");
    let rec = Recording::read(&path).unwrap_or_else(|e| panic!("{e}"));
    let rule = |key: &str| match rec.rules.get(key) {
        Some(Value::Float(x)) => *x,
        other => panic!("rules.{key} is {other:?}"),
    };
    (rule("bpm"), rule("t_fight"))
}

/// What `trigger` does for the FLUTE's unlocks, at the end of the step that fires them: FLUTTER TONGUE is
/// `B.haste = ab_dur(ab); B.haste_mult = ab['mult']`, SNAKE DANCE is `B.dance_until = self.t + ab_dur(ab)`, with
/// `ab_dur(ab) = ab['dur_beats'] * beat`.
fn fire_by_hand(fight: &mut Fight, k: usize, name: &str) {
    let (cfg, beat) = (fight.balls[k].cfg, fight.rules.beat);
    let dur = |slot: Slot| slot.of(cfg).get("dur_beats").expect("the ability lasts some beats") * beat;
    match name {
        "FLUTTER TONGUE" => {
            fight.balls[k].haste = dur(Slot::Unlock1);
            fight.balls[k].haste_mult = Slot::Unlock1.of(cfg).get("mult").expect("haste has a factor");
        }
        "SNAKE DANCE" => {
            let until = fight.t + dur(Slot::Unlock2);
            fight.striker[k].as_mut().expect("the FLUTE is a striker").dance_until = until;
        }
        other => panic!("{other} is not one of the FLUTE's unlocks"),
    }
}

/// The largest difference between two lists of floats; panics past `TOL`.
fn compare_floats(what: &str, rs: &[f64], py: &[f64]) -> f64 {
    let mut worst = 0.0f64;
    for (a, b) in rs.iter().zip(py) {
        let d = (a - b).abs();
        assert!(d <= TOL, "{what}: {a} vs Python's {b} ({d:e} apart)");
        worst = worst.max(d);
    }
    worst
}

/// The largest float difference between two values of the same layout; panics where the layouts differ or a float
/// differs by more than `TOL`.
fn compare_values(what: &str, a: &Value, b: &Value) -> f64 {
    match (a, b) {
        (Value::List(x), Value::List(y)) => {
            assert_eq!(x.len(), y.len(), "{what}: {a:?} vs Python's {b:?}");
            x.iter()
                .zip(y)
                .map(|(p, q)| compare_values(what, p, q))
                .fold(0.0, f64::max)
        }
        (Value::Float(x), Value::Float(y)) => {
            let d = (x - y).abs();
            assert!(d <= TOL, "{what}: {x} vs Python's {y} ({d:e} apart)");
            d
        }
        _ => {
            assert_eq!(a, b, "{what}");
            0.0
        }
    }
}

/// What one comparison covered.
struct Span {
    /// Events matched, by kind (`hit` are the cobras' bites).
    kinds: BTreeMap<String, usize>,
    /// Abilities fired by hand.
    abilities: usize,
    /// Sampled frames in which a cobra is off its basket.
    frames_off_basket: usize,
    /// The FLUTE's reach in the last frame compared.
    reach: f64,
}

/// Runs the Rust fight of the vectors' seed, firing the FLUTE's unlocks by hand when Python did, and compares every
/// event and every sampled frame before `t_end`.
fn compare(v: &Vectors) -> Span {
    let name = v.name;
    let roster = parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads");
    let flute = roster.iter().find(|f| f.name == "FLUTE").expect("in the roster");
    let bag = bag();
    let (bpm, t_fight) = tempo();
    let mut rules = Rules::new(bpm, t_fight);
    rules.lv_dmg = (2.0, 5.0);
    rules.low_hp = -1.0;
    let fighters = if v.flute == 0 { [flute, &bag] } else { [&bag, flute] };
    let setup = Setup {
        fighters,
        rules,
        seed: v.seed,
    };

    let py_events = events(v.events);
    let py_frames = frames(v.frames);
    let mut abilities = py_events.iter().filter(|e| e.kind == "ability").peekable();

    // the fight loop (`Fight.run`), with a frame after every `steps_per_frame` steps
    let mut fight = Fight::new(&setup, false);
    let mut sched = Schedule::new(&mut fight);
    fight.t = fight.rules.t_fight;
    let mut rs_frames = vec![snap(&fight)];
    let mut fired = 0;
    while fight.t < v.t_end {
        sched.step(&mut fight);
        let t = bf_core::py::round_nd(fight.t, 5);
        while let Some(e) = abilities.next_if(|e| e.t <= t) {
            assert_eq!(e.t, t, "{name}: Python fired {e:?} between two steps");
            assert_eq!(e.who(), Some(v.flute), "{name}: only the FLUTE has abilities: {e:?}");
            fire_by_hand(&mut fight, v.flute, e.s(3));
            fired += 1;
        }
        assert!(fight.balls.iter().all(|b| b.hp > 0.0), "{name}: no K.O. in the window");
        if sched.step % sched.steps_per_frame == 0 {
            rs_frames.push(snap(&fight));
        }
    }
    assert!(abilities.next().is_none(), "{name}: every Python ability was fired");

    // every event before t_end, but the abilities: in order, the same kinds, times and whole numbers
    let wanted = |events: &[Event]| -> Vec<Event> {
        let keep = |e: &&Event| e.t < v.t_end && e.kind != "ability";
        events.iter().filter(keep).cloned().collect()
    };
    let (rs, py) = (wanted(&fight.events), wanted(&py_events));
    let (mut kinds, mut max_dev) = (BTreeMap::new(), 0.0f64);
    for (i, (a, b)) in rs.iter().zip(&py).enumerate() {
        let what = format!("{name}: event {i}:\n Rust   {a:?}\n Python {b:?}\n");
        assert_eq!((a.t.to_bits(), &a.kind), (b.t.to_bits(), &b.kind), "{what}");
        let (a_fields, b_fields) = (Value::List(a.fields.clone()), Value::List(b.fields.clone()));
        max_dev = max_dev.max(compare_values(&what, &a_fields, &b_fields));
        *kinds.entry(b.kind.clone()).or_insert(0) += 1;
    }
    assert_eq!(
        rs.len(),
        py.len(),
        "{name}: events before {} s: after the first {}, Rust has {:?} and Python {:?}",
        v.t_end,
        rs.len().min(py.len()),
        rs.get(py.len()),
        py.get(rs.len())
    );

    // the sampled frames
    let (mut max_dp, mut max_dv, mut max_dhp, mut max_dcob) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let (mut frames_off_basket, mut reach) = (0, 0.0);
    for (i, py) in &py_frames {
        let rs = rs_frames
            .get(*i)
            .unwrap_or_else(|| panic!("{name}: the Rust fight has no frame {i}"));
        let what = |key: &str| format!("{name}: {key} at frame {i}");
        max_dp = max_dp.max(compare_floats(&what("P"), &rs.p, &py.p));
        max_dv = max_dv.max(compare_floats(&what("V"), &rs.v, &py.v));
        max_dhp = max_dhp.max(compare_floats(&what("HP"), &rs.hp, &py.hp));
        compare_floats(&what("BL"), &rs.bl, &py.bl);
        max_dcob = max_dcob.max(compare_values(&what("COB"), &rs.cob, &py.cob));
        let Value::List(sides) = &py.cob else {
            panic!("{name}: COB at frame {i} is {:?}", py.cob)
        };
        let Value::List(baskets) = &sides[v.flute] else {
            panic!("{name}: COB at frame {i} is {:?}", py.cob)
        };
        // a row is [id, home x, home y, cobra x, cobra y, t_drop]
        let off = |b: &Value| matches!(b, Value::List(f) if f[1] != f[3] || f[2] != f[4]);
        frames_off_basket += usize::from(baskets.iter().any(off));
        reach = py.bl[v.flute];
    }
    println!(
        "{name}: agrees until {} s: {} events {kinds:?} with max |d| {max_dev:.3e}, {fired} abilities fired by hand; {} \
         frames with max |dP| {max_dp:.3e}, |dV| {max_dv:.3e}, |dHP| {max_dhp:.3e}, |dCOB| {max_dcob:.3e}",
        v.t_end,
        py.len(),
        py_frames.len(),
    );
    Span {
        kinds,
        abilities: fired,
        frames_off_basket,
        reach,
    }
}

/// The FLUTE in the left corner, seed 9, for 9.9 s of fighting: 6 baskets (4 taken back), 19 lunges of which 18 bite,
/// both levels, FLUTTER TONGUE (lunges on the 8th notes, twice as quick) and SNAKE DANCE (the cobras off their
/// baskets in 6 of the frames compared), the reach grown to its cap of 2.4.
#[test]
fn flute_on_the_left_matches_python() {
    let span = compare(&LEFT);
    let count = |kind: &str| span.kinds.get(kind).copied().unwrap_or(0);
    assert_eq!(
        (count("basket"), count("strike"), count("hit"), count("level")),
        (6, 19, 18, 2)
    );
    assert!(count("rope") >= 20 && count("clash") >= 3, "{:?}", span.kinds);
    assert_eq!(span.abilities, 2, "FLUTTER TONGUE and SNAKE DANCE");
    assert!(span.frames_off_basket >= 5, "{}", span.frames_off_basket);
    assert_eq!(span.reach, 2.4, "the reach is at max_reach");
}

/// The FLUTE in the right corner, seed 1: every event names fighter 1 as the charmer and fighter 0 as the bitten,
/// and the baskets are the second list of a `COB` row.
#[test]
fn flute_on_the_right_matches_python() {
    let span = compare(&RIGHT);
    let count = |kind: &str| span.kinds.get(kind).copied().unwrap_or(0);
    assert_eq!(
        (count("basket"), count("strike"), count("hit"), count("level")),
        (6, 11, 10, 2)
    );
    assert_eq!(span.abilities, 2, "FLUTTER TONGUE and SNAKE DANCE");
    assert!(span.frames_off_basket >= 2, "{}", span.frames_off_basket);
}

/// The vectors are what they say: Python's events in time order, the bag never acting, and the abilities the two
/// the test fires by hand.
#[test]
fn the_vectors_hold_a_striker_only_fight() {
    for v in [&LEFT, &RIGHT] {
        let py = events(v.events);
        assert!(py.windows(2).all(|w| w[0].t <= w[1].t), "{}", v.name);
        assert!(py.iter().all(|e| e.t < v.t_end), "{}", v.name);
        for e in &py {
            match e.kind.as_str() {
                // the charmer; a bite's victim is the bag
                "basket" | "strike" | "level" | "ability" => assert_eq!(e.who(), Some(v.flute), "{e:?}"),
                "hit" => assert_eq!(
                    (e.who(), e.i(3), e.s(7)),
                    (Some(v.flute), 1 - v.flute as i64, "fang"),
                    "{e:?}"
                ),
                "rope" | "clash" => {}
                other => panic!("{}: a {other} event", v.name),
            }
        }
        let names: Vec<&str> = py.iter().filter(|e| e.kind == "ability").map(|e| e.s(3)).collect();
        assert_eq!(names, ["FLUTTER TONGUE", "SNAKE DANCE"], "{}", v.name);
        assert_eq!(frames(v.frames)[0].0, 0, "{}: the first frame listed is f0", v.name);
    }
}
