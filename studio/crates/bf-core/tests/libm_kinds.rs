//! Python's maths, held to the bit (ruling M3-R38): `bf_core::dmath` gives what CPython 3.13.18's `math` module
//! and numpy 2.5.3's `float64` functions give on the reference PC. The tables are a sample of the 2 to 6 million
//! arguments each function was measured on (all equal): for each function 14 ordinary arguments where the maths
//! library `dmath` used before (musl's) was one step from Python, then 10 drawn without choosing. `math.tanh` and
//! `np.tanh` are two functions (numpy has its own): the table holds both and the first 14 rows are arguments
//! where they differ.

use bf_core::dmath;

#[rustfmt::skip]
const SIN: [(u64, u64); 24] = [
    (0x400d051f2383dc44, 0xbfdde3848a0c98fd),
    (0x400e15b6143b6630, 0xbfe2913a3387f724),
    (0xc01b288406cdbed8, 0xbfdf0a7e3d811704),
    (0xc00a1d1419eb602e, 0x3fbf4efb148c5831),
    (0xc00d811f2f33a06d, 0x3fe0a111df9e4509),
    (0x3fd7fc540ba737a0, 0x3fd76d97a913ce1b),
    (0x3fd6d1d4d51b2f00, 0x3fd656d59399442f),
    (0x3fe880379c1c74a8, 0x3fe62d1cb08429ab),
    (0x400c57f27480064c, 0xbfd90099c38af99b),
    (0x4016c053272273d8, 0xbfe1f22f482ea5b9),
    (0x3fe0060edf7fad38, 0x3fdeb989f144337d),
    (0x401b94a7c01246f2, 0x3fe262415092e47a),
    (0xc0022e00e3a6cb96, 0xbfe870c8e10cc7f3),
    (0xc0178d3fce2df55a, 0x3fd8a46d21d840f8),
    (0xbff541debe516b5c, 0xbfef10de537f30fc),
    (0xc0a38f061e9cff0c, 0xbfd52f8f52a0ef89),
    (0xb27404223023c526, 0xb27404223023c526),
    (0xdfea26b617eb10e2, 0x3feeb099d94b8058),
    (0xbff175467c18e110, 0xbfec6387d3e2ed62),
    (0x40a7b8e8d99c70ce, 0x3fefd27719851aad),
    (0x0bfdf69416b80731, 0x0bfdf69416b80731),
    (0x3ff6dd97ed085bb0, 0x3fefade60a782d37),
    (0x40aafcc2bbfb2ae8, 0xbfef5df9d8b20066),
    (0x9bed21545a034994, 0x9bed21545a034994),
];

#[rustfmt::skip]
const COS: [(u64, u64); 24] = [
    (0xbffd0fd927c02760, 0xbfcf1e4bbb564616),
    (0x3fefe6dcddcd2038, 0x3fe15f49b25326ff),
    (0xbfef38678e6c73f0, 0x3fe1f0c7acaf0282),
    (0x3ff15fc3a21f1a00, 0x3fddd5255cef41e2),
    (0x4013ccf6f4d85aac, 0x3fce2619bb8c0bfb),
    (0x3ff6ca74401df3d8, 0x3fc2ab1d29b3e840),
    (0x40024d1729370dac, 0xbfe5063979a7aa08),
    (0xbffc674270da3fd8, 0xbfc9fbac14a79fdb),
    (0x40028b0593c87888, 0xbfe5be7daf83c76e),
    (0xc015bb46795be59a, 0x3fe51cbd60cb731b),
    (0x3fe112b07d1f5c58, 0x3feb8d725acfb5fc),
    (0x3fdfd6428f1e6da0, 0x3fec1f237c7ad131),
    (0xc011a509e4831c77, 0xbfd2fcdee150cefc),
    (0xc015d7d9a8f017f6, 0x3fe5c666993cc25e),
    (0xbff1316cf313ca80, 0x3fde78a3ae32e58e),
    (0xc0a86500d514aae8, 0x3fef1253626f5f2d),
    (0xda6e56c410affb5e, 0x3fe520609906aed2),
    (0x4ee9b486afac8678, 0xbfed77be5c2195a7),
    (0xc00360d7844cea72, 0xbfe81289615bd40f),
    (0x40b2ee803dd87666, 0xbfe1eac583bebaf7),
    (0x3f06064b23250a6f, 0x3fefffffff86babc),
    (0x3fc5bd32e0509440, 0x3fef8a23166232af),
    (0xc0a0e7aa958edbea, 0xbfe7f83c4dcfd27a),
    (0x985972edf549f1b5, 0x3ff0000000000000),
];

#[rustfmt::skip]
const TAN: [(u64, u64); 24] = [
    (0x4014b6fb43f1150e, 0xbfffc9e2f7ee97cf),
    (0x401b269191dcfafc, 0x3fe1ab12def21860),
    (0xc01118ec503bdfb5, 0xc001145387208118),
    (0x40108bbe4128190a, 0x3ff8a3ce73125689),
    (0x40164061bcc4aeba, 0xbfec1578657ce892),
    (0xc01aac23dad948d4, 0xbfd9eda2cae6e41d),
    (0x40171328af6d6c9a, 0xbfe216857bf80064),
    (0x401a8587897c9dd6, 0x3fd728f6188323a6),
    (0xc011791c02a807fe, 0xc00652a6ee88d0c2),
    (0x4016dbf87ec57576, 0xbfe46fff4cabe8eb),
    (0x400345071d02b244, 0xbfeccde0500e7ae8),
    (0x40165cc74976b0fa, 0xbfea8cc7b5b9d12e),
    (0x400341fd8a9afb6c, 0xbfece3e6330dc589),
    (0xc01534b8aab9e673, 0x3ff7f1758188f27e),
    (0x401580d7dcd78b00, 0xbff4786112802bdb),
    (0x40afa9eccb5846f8, 0x3fd45b855f5513c8),
    (0xe6d39a31e16c693a, 0x3fec984193e823b7),
    (0xc0114a79acf40782, 0xc0037b22a92af251),
    (0xc0b0e27489b3e8df, 0x3fd949213f7736a3),
    (0xb3a245e7e5a424ba, 0xb3a245e7e5a424ba),
    (0x401580efa8cef50c, 0xbff477661bdce60d),
    (0x4099e4657af7509c, 0x4025e96a949f64a9),
    (0x8edb8857e0241a94, 0x8edb8857e0241a94),
    (0x4016c0b465cb1582, 0xbfe5a87cf24b4a5d),
];

#[rustfmt::skip]
const ASIN: [(u64, u64); 24] = [
    (0xbfe2502b98e8a32a, 0xbfe37f506c2baec8),
    (0xbfef86aee46f348a, 0xbff66030155174d8),
    (0x3fe1289d86d7a9d4, 0x3fe21c2a50326812),
    (0xbfefb88013a3bb94, 0xbff7046b5dc88122),
    (0x3feff48648520eea, 0x3ff8492750ec49ec),
    (0x3fd46fe67efff790, 0x3fd4cd2a0f90e2c8),
    (0x3fe3b2d92ac4ed84, 0x3fe538508f684d26),
    (0x3fe4eb371a7f3300, 0x3fe6cca3adf3a5be),
    (0xbfe252463b58db50, 0xbfe381e15296e107),
    (0x3fd65a1e3e4e2710, 0x3fd6d555f551e341),
    (0x3fda13e9cc2dd7a8, 0x3fdadbfc7c76a691),
    (0xbfdda5c2ec39a764, 0xbfded35b66fc5cdf),
    (0xbfe253abbc9060c4, 0xbfe383956704a1b2),
    (0xbfd4e05e5d26531c, 0xbfd543fe170b5a76),
    (0x3feb40b6ad19e2bc, 0x3ff04e55ba7038c0),
    (0x3e35987f9f2e861a, 0x3e35987f9f2e861a),
    (0xbfea76f817424346, 0xbfef294add6fd852),
    (0x02d64bad660acef7, 0x02d64bad660acef7),
    (0x3fdfd1f43c8da5a8, 0x3fe0a6c2e71f4f22),
    (0x16787f85d0c02635, 0x16787f85d0c02635),
    (0xbfe18d0dbeec2868, 0xbfe293b6deaa064a),
    (0xa2b5ff9fcd44a1d8, 0xa2b5ff9fcd44a1d8),
    (0x3fda24946961c8ec, 0x3fdaee3db352836d),
    (0x3422f1944c99a236, 0x3422f1944c99a236),
];

#[rustfmt::skip]
const ACOS: [(u64, u64); 24] = [
    (0x3fdcc9cd70f0f60c, 0x3ff1aaf33484d715),
    (0x3fe1b2534c4381dc, 0x3fef83984b0b0fe6),
    (0xbfe2aae58e07148e, 0x40018c9c4b855e20),
    (0xbfe549d2d3a60b62, 0x4002639da92a6a9b),
    (0xbfe42f26ddf246da, 0x400206ce4f9ec9cc),
    (0x3fe2d7154cf6668c, 0x3fee1ef395fab5b2),
    (0x3fe08d8011f6c2d6, 0x3ff06f24ff22c7c7),
    (0xbfe3f9e93faa68ee, 0x4001f5b6040e3f8e),
    (0xbfe6c6d338e9401c, 0x4002e70a0b86570d),
    (0x3fe0ab8e8cc088ec, 0x3ff05d8fc2ef50d0),
    (0xbfe539f85db79bc0, 0x40025e50434625a4),
    (0x3fe12a19a6ae28fe, 0x3ff01304fce93c3c),
    (0xbfe1f46b7c04efda, 0x400154fad0a394b2),
    (0xbfe173eb15cbc4a8, 0x40012e69d742b675),
    (0x3f9e32c5066ccbc0, 0x3ff8a92bc44207e2),
    (0xadf223a1af5de882, 0x3ff921fb54442d18),
    (0xbfdc733adf3a97ec, 0x4000406878e326ea),
    (0x98646c46e3a0ef7a, 0x3ff921fb54442d18),
    (0x3fe981007d1f3696, 0x3fe4c06c150a6135),
    (0x37a64829e275f8b7, 0x3ff921fb54442d18),
    (0x3fd7aa2d360d6984, 0x3ff3129c74031bb2),
    (0x30fb5183bcf2aa89, 0x3ff921fb54442d18),
    (0x3fe029f545279a08, 0x3ff0a90e15775295),
    (0x2df96834e8cbc600, 0x3ff921fb54442d18),
];

#[rustfmt::skip]
const ATAN: [(u64, u64); 24] = [
    (0x4019906446e41458, 0x3ff6a63c6f95e50f),
    (0xbf91e2eadeade600, 0xbf91e273ad7a2e3f),
    (0xc0173303c95da540, 0xbff666a102475ccc),
    (0x3fc4ce91e3eb14c0, 0x3fc4a06244e13287),
    (0x4007192e0862ddc8, 0x3ff3cc5069befac2),
    (0xbfdb9b169d0fdcc0, 0xbfda100ab3c5165e),
    (0xbff302be51a77a88, 0xbfebe0b788b83c25),
    (0xc00b9a22f64c5a76, 0xbff49e7c72087d89),
    (0x4013ee0908bccd38, 0x3ff5f6ad2783cf43),
    (0xbfda50a706c07940, 0xbfd8f768ecf47189),
    (0x3fc6616b7a134000, 0x3fc62814e9aeb9d4),
    (0x4018e92a840f8db8, 0x3ff695d78e1d2521),
    (0x3fd971cc77318080, 0x3fd837e054eb7e0f),
    (0xc007d47485e32088, 0xbff3f3539d8c88a9),
    (0xc02267e17c81b616, 0xbff766a4c983d194),
    (0x4bfb3da218ddb8fd, 0x3ff921fb54442d18),
    (0xc0113363949665ae, 0xbff57a192a7ef1f6),
    (0x970a510b6261086f, 0x970a510b6261086f),
    (0xc00b39d643a75440, 0xbff48f5ef79467aa),
    (0xb1e5ff27c38300b3, 0xb1e5ff27c38300b3),
    (0xbff2f41fa129d390, 0xbfebd4925a64bbae),
    (0x7b5dcd8b3bea8637, 0x3ff921fb54442d18),
    (0x4020b14ac037b934, 0x3ff7398d582f7912),
    (0x10169c58e4ae9968, 0x10169c58e4ae9968),
];

#[rustfmt::skip]
const EXP: [(u64, u64); 24] = [
    (0xc025b6d99d993ae8, 0x3ef433e1ee9bd3e1),
    (0x40324493438c86c2, 0x4194769c3f6c9fe5),
    (0xbffca96a250fb5a0, 0x3fc557828de3d9f3),
    (0xbfd6eba98dcf0d40, 0x3fe65e0a84876053),
    (0x4032a1769aad3a0a, 0x419d6a09f15ec8dd),
    (0xc02419e24c313f7e, 0x3f06a11444027417),
    (0x4027302348219fa2, 0x40fa79f408d2d98f),
    (0x402fadc6e5ca2b50, 0x415cde64c9cc2b59),
    (0xc02f092a86602662, 0x3e8875f6258aa9a9),
    (0xc00cfb891c352068, 0x3f9b59256510909f),
    (0xc0227da2a11376b8, 0x3f194fcd6f3f09d1),
    (0xc03199979821f84d, 0x3e58658b2a265d8f),
    (0x3ff0f74cf1d1bdd0, 0x40071982f34e648b),
    (0xc01989cc35b3a520, 0x3f5ba574f0076c5d),
    (0x40001cee7bb450c8, 0x401dfa0a5fa94e0a),
    (0x4081980f1d4bca4a, 0x72b30017f05648cb),
    (0xb73dbd14554bfa01, 0x3ff0000000000000),
    (0x4026d73b7c9869e0, 0x40f641862edd6dbf),
    (0xc07872fb6aea8eb7, 0x1ca8e33141efb866),
    (0xb6afb67e70691c62, 0x3ff0000000000000),
    (0xc01a47c0865e74de, 0x3f56f7269b990beb),
    (0xc0706ff4a0f03223, 0x2837d69ad7bb65ed),
    (0x3f6a7bea247cc927, 0x3ff00d437164a323),
    (0x402a321091594c34, 0x411dc6cdd748f6f8),
];

#[rustfmt::skip]
const LN: [(u64, u64); 24] = [
    (0x40a1ecf4f3c084de, 0x401ef3faa59ceacb),
    (0x40b788aa67653f6d, 0x40216840858d5af3),
    (0x40c35903ff781bfb, 0x402266dc3e0dec21),
    (0x4075a1153609e9d9, 0x401762f41e05ac4f),
    (0x40b50a82639f1d9e, 0x40212eed1ed12bb3),
    (0x40a7bb5b5f703fcc, 0x402009a68e1642e9),
    (0x40b72fee51333a13, 0x402160a7a9192ea3),
    (0x40a2c5511f3cae05, 0x401f2327b043c389),
    (0x4095fd129910f586, 0x401cff643086575f),
    (0x40b5ecc0fe076e77, 0x402143fdf00626c9),
    (0x406ccf5ab4c23564, 0x4015c2ba2c6dcadf),
    (0x40bd301c59b3903a, 0x4021d67e5c2eb623),
    (0x40c18c368b8f38b5, 0x402234dce992ec1d),
    (0x409b9435e6a90a61, 0x401de75bb6565a1d),
    (0x40c009e6d7078d03, 0x402206d2e8295b97),
    (0x25a95062ee72c65f, 0xc07235b2f839b110),
    (0x3ff8f4c335c605ae, 0x3fdc732cb96b84be),
    (0x40c0dd8e7eede66d, 0x4022208f24fb5088),
    (0x18246187a6c430a7, 0xc07b94af10fe6d4d),
    (0x3ffd7976d3684f19, 0x3fe38cc91232f9f2),
    (0x40b9b10ff152cd68, 0x4021952af8628dbc),
    (0x5c1b0338342467df, 0x4073870a03d9a355),
    (0x3fffa41126238a5c, 0x3fe5d1cf1753e281),
    (0x408d5d58671585c4, 0x401b61d1c6da6db5),
];

#[rustfmt::skip]
const LOG10: [(u64, u64); 24] = [
    (0x4095a1b0d634295a, 0x400921517d304815),
    (0x40b6f2f4521d9dbf, 0x400e26ebd49a2d5a),
    (0x40b4c12e2847572d, 0x400dcd86b037a3b1),
    (0x40a6139725e75689, 0x400b9befa5b85792),
    (0x40a73361cb318a09, 0x400bc81cd7c21b62),
    (0x40b65ae665c65cf0, 0x400e0f990f39fe4d),
    (0x40a65afceaf7faae, 0x400ba71a252669d6),
    (0x40c34bf72f8c9886, 0x400ff5418abead04),
    (0x40bb91ac17b8bfac, 0x400eca0f086f031d),
    (0x40bc90d3f3cacda0, 0x400ee9a5952b0881),
    (0x4093ac196eff5429, 0x4008ccdec82ae881),
    (0x40b6bd911549477f, 0x400e1ecd3a86be78),
    (0x40a74d3b773d3017, 0x400bcbf9b3f1b2f2),
    (0x40bd1001d4cf6fc8, 0x400ef8fb733d8d52),
    (0x40c155fdf157cae5, 0x400f95f2893e1e4a),
    (0x55ae9a0fa6d7ade5, 0x405a2f4c5b921a44),
    (0x3fec40fa8d20312a, 0xbfabaf6ca19baabc),
    (0x40b32ffb71efd50e, 0x400d87b1a32547ca),
    (0x490803ef54824c10, 0x4045e9b126580508),
    (0x3ff8eb08f34e00d0, 0x3fc8a0668687465b),
    (0x40a78e9458e220db, 0x400bd5aa780fb7d7),
    (0x120e4ed096b0fedd, 0xc06b9f58f9f48fc8),
    (0x3ff72968a6a44389, 0x3fc49039ff590f95),
    (0x408ad800934879c4, 0x400778d17b7ac9d5),
];

#[rustfmt::skip]
const LOG2: [(u64, u64); 24] = [
    (0x40a950ea8259ce94, 0x402752ef4f0e527e),
    (0x40c1fcbcb8a12137, 0x402a567a77b6ca92),
    (0x40a703f86e436b8d, 0x40270c8fc8615c86),
    (0x40a967660883ffe3, 0x4027557e283f76e2),
    (0x40b433d39fd87760, 0x4028ac4440017181),
    (0x40a850063dbb6226, 0x4027350f40c378c4),
    (0x40bb8f3d8a39ec74, 0x402991a74cddeed7),
    (0x40acebbe60c203a3, 0x4027b544a14d8e28),
    (0x40c2fcb4a5ed9003, 0x402a7e703ceeadbb),
    (0x4015c7b1c1b9f654, 0x40038f3a7b80fce0),
    (0x40b4565c601b5c22, 0x4028b12eb8fa37b0),
    (0x4091f015af445946, 0x402454722b370032),
    (0x40b4242e8e5a7bca, 0x4028aa075d33ed7c),
    (0x40b7b057dc58652b, 0x402921dc89764c4f),
    (0x40ba74f4bffb19bd, 0x4029737de687f225),
    (0x25ed46e7b7a5071f, 0xc07a020d8a103233),
    (0x3ff24695b825132d, 0x3fc88ec0f89d1389),
    (0x40bd0b450cd5b0de, 0x4029b8681e6fc51b),
    (0x200726d040842356, 0xc07fe778aa0fc811),
    (0x3ffefb8a48af0670, 0x3fee8221f0db6551),
    (0x40baa2697add3bcb, 0x4029786ec3c3ca27),
    (0x1470f0371a714170, 0xc085bf579cb22193),
    (0x3ffbc4fb2042a6ea, 0x3fe9742370dd40ec),
    (0x40bc4a4a01c73a67, 0x4029a4fb56518b9c),
];

#[rustfmt::skip]
const SINH: [(u64, u64); 24] = [
    (0x3fe38fe94525e490, 0x3fe4cdb55a7ea361),
    (0x3ffed88079c965f0, 0x400aeace05998455),
    (0xbfe3922a4b8922e0, 0xbfe4d065a64bb3ab),
    (0xbfe8fee5ec6d0bf0, 0xbfeb9dbedce0503f),
    (0xc0028944ef459190, 0xc01418020d779ccf),
    (0x3ffc4c5c53b65f14, 0x4006c4f12510385c),
    (0x3fcc6ac87faebb00, 0x3fcca6b0c00d6426),
    (0xc008dc1584b1cd6c, 0xc02651cbf21dacad),
    (0xbfebfa1d65872968, 0xbfefae0afccd9ae7),
    (0x3ffd9e3ff4d6e9d0, 0x4008d70633ce29b2),
    (0xbff53c786448d9b0, 0xbffc0b47a078c560),
    (0xbf7bcf8ce03f0800, 0xbf7bcf9ae13b9cca),
    (0xbffd90d4bb2abeba, 0xc008c12bfcf86146),
    (0x3ff4d2618450ccfc, 0x3ffb3794984cab14),
    (0xbfd693b7a20a3cb0, 0xbfd70c572641c887),
    (0x3ff211369afcc950, 0x3ff628c0135f9625),
    (0x9fa644dcb2298729, 0x9fa644dcb2298729),
    (0xbffc5bea6f1441b0, 0xc006dc726293fca2),
    (0x400aea74afeabdc0, 0x402ce24665a65236),
    (0x1e40c1033bf9fb09, 0x1e40c1033bf9fb09),
    (0xbff2b7de95ac8a48, 0xbff74a36b848ce9c),
    (0xc01f11ea728854a4, 0xc092751846d42b82),
    (0x0a6769bcf542ace8, 0x0a6769bcf542ace8),
    (0x400d719c774e6ed8, 0x4033d1eb24920372),
];

#[rustfmt::skip]
const TANH: [(u64, u64, u64); 24] = [
    (0x3ff8348b5a9714dc, 0x3fed09bee81f2672, 0x3fed09bee81f2673),
    (0x400cfd1dace0d7e4, 0x3feff457098bc770, 0x3feff457098bc76f),
    (0x4012f32106412926, 0x3feffebe14bdfd7d, 0x3feffebe14bdfd7c),
    (0xbffcd1ac4ebbd9cc, 0xbfee4d3e365a91c3, 0xbfee4d3e365a91c4),
    (0x3fd6e690b3933090, 0x3fd5f88a3355094a, 0x3fd5f88a3355094b),
    (0xc006544e9d149a7a, 0xbfefc290c0258da2, 0xbfefc290c0258da3),
    (0xc00db997173c507a, 0xbfeff64c8bfebc6e, 0xbfeff64c8bfebc6d),
    (0x4011a72228caf76a, 0x3feffd985cbda4d9, 0x3feffd985cbda4d8),
    (0xbff1895d850cf198, 0xbfe991fce5da1fec, 0xbfe991fce5da1feb),
    (0xbffb7ea15564b7ee, 0xbfee0168a1f830d1, 0xbfee0168a1f830d2),
    (0x40053664b08ea8f6, 0x3fefaee0443d3b37, 0x3fefaee0443d3b38),
    (0xc00f62a05e856406, 0xbfeff997de38bdec, 0xbfeff997de38bdeb),
    (0x40080e74e92a17d8, 0x3fefd80e1956bf42, 0x3fefd80e1956bf41),
    (0x3fe4904924e04b28, 0x3fe22235e5432cd5, 0x3fe22235e5432cd4),
    (0xbfe6fd7d5b7ef830, 0xbfe3b5d0ca0b4966, 0xbfe3b5d0ca0b4966),
    (0x400d29125dfd02e0, 0x3feff4d45f3bb3e6, 0x3feff4d45f3bb3e5),
    (0x1b62be2f705810bf, 0x1b62be2f705810bf, 0x1b62be2f705810bf),
    (0xc0059387f70ed7ec, 0xbfefb5e5ab698a7c, 0xbfefb5e5ab698a7d),
    (0x403917db295ad9c2, 0x3ff0000000000000, 0x3ff0000000000000),
    (0xb6841fce3989e779, 0xb6841fce3989e779, 0xb6841fce3989e779),
    (0x4007088088034b36, 0x3fefcc727cb50b24, 0x3fefcc727cb50b24),
    (0xc0248324bf9a8bc8, 0xbfeffffffeac8df4, 0xbfeffffffeac8df4),
    (0x95584f7145b33e0d, 0x95584f7145b33e0d, 0x95584f7145b33e0d),
    (0x401125f2c7b1d95c, 0x3feffce7b5036f95, 0x3feffce7b5036f95),
];

#[rustfmt::skip]
const ATAN2: [(u64, u64, u64); 24] = [
    (0xc0891fc01b18efcb, 0xc07158291145a158, 0xbffe735e91f27150),
    (0x40966ce7e0a25d9a, 0x4094887a4c8da464, 0x3fea8a99eadb5806),
    (0x4096cb4ea36ceee6, 0x40823b7ccb963070, 0x3ff30ba834eba2b3),
    (0xc0970733e3fff49d, 0xc084de5488dd798e, 0xbffff0910e28c6cb),
    (0x407af35c37a4cfc0, 0xc094f8dc33f706ad, 0x4006a55b39f42875),
    (0xc087cb9fc7375192, 0xc05b7df5714efae0, 0xbffb6d758e08559e),
    (0xc07d58d8d204f7dc, 0x4091f59accc2aa52, 0xbfd8d231b795599b),
    (0xc0920769b84470a1, 0xc0961e8b2cd5c7d6, 0xc003a96bec9f96e7),
    (0xc080b1589c9b717c, 0x408c21a3f53d6340, 0xbfe12327baaeea4e),
    (0x4079cfaa821306e0, 0x4094ac4e749ff3e8, 0x3fd35d061bdf930b),
    (0x408cd1677ec540c8, 0x4083765ebd7a6418, 0x3fef41f75b96a862),
    (0x4093f8cdcd045d3e, 0xc082bb7ed9f2050d, 0x400013127e1a1acb),
    (0xc090eb901bb6a378, 0xc068616884f972c8, 0xbffbfbe74eaf2e3f),
    (0x4090c638ea72548c, 0x407dc975e93aed44, 0x3ff272ab8bda1ae4),
    (0x4080d6de9945dda0, 0xc090b71f155e0841, 0x4005665cd2096366),
    (0x95d782e882583006, 0x03bc59818d1607f9, 0xbff921fb54442d18),
    (0x3fe381b2d454a844, 0x3fe555d68f16ba58, 0x3fe7b36f7113f178),
    (0xbf0523fb5d2b330b, 0x3f13394704d4b39b, 0xbfe01661ce2d5386),
    (0x4096dcfdcd71e044, 0xc0914b26dcb2f210, 0x4001bf3b918b1d12),
    (0xa36ed5f8f211dd0a, 0x33436c124d3cb0ab, 0xb0196711009cd4fd),
    (0xbfe77c8976a4ed56, 0x3fdae1f9d6c56334, 0xbff0d0dce1dd9948),
    (0x3f2d3b9f41bc424e, 0xc06455493c1aa64f, 0x400921fa9c3dbbe2),
    (0xc069a791933e1950, 0xc094789fe56591e8, 0xc007e3bf79572a38),
    (0xcec576c5e213fc6c, 0x131c1dc7e3fb586c, 0xbff921fb54442d18),
];

#[rustfmt::skip]
const POW: [(u64, u64, u64); 24] = [
    (0x4001272fdbbc6bb7, 0x40099d5b2f48fdf8, 0x4026fedc8daf15bf),
    (0x405432420bbe8130, 0xbfe23985670d8da0, 0x3fb4fcf3c9c182a3),
    (0x401f842321bded5d, 0x400d3cf9156ad548, 0x409d87055c054b61),
    (0x4053b303976ad3c7, 0x3ffd5f04d4522934, 0x40a7abaee188ade5),
    (0x4035d7d010fccd46, 0x40136d3ddbd1a106, 0x4148624d34753455),
    (0x40556ca859b8642f, 0x400d28996d2b7ac4, 0x41652c0de06ab9e1),
    (0x403663b3a58845f1, 0x3ff6d07d52fa221c, 0x405509476b103c23),
    (0x4029de21a40de9bd, 0x3ffc0f42f29960a0, 0x4056439ed1e4a9b3),
    (0x404983972223bf80, 0x3fb6bcc5e5939e00, 0x3ff6b03bab45b7a5),
    (0x40487b9c4a2f31d2, 0xc00939c0e6a2f586, 0x3ed3aedc70e64dcd),
    (0x404303496802da41, 0xbff72e835cfe8d98, 0x3f750a5f2cdbba67),
    (0x4031000dcaaaf2f9, 0xbfea2157b2aa3490, 0x3fb9524754f11f4f),
    (0x4008167f1469a0e8, 0x40029c66d0b574b0, 0x4029fb8a0baecd73),
    (0x4055161b22eb76d2, 0x3ff5a9de91172cf0, 0x407955a584704e75),
    (0x40500776e89d6423, 0xc00e23e41560b388, 0x3e84e4a1eda50b69),
    (0x3ffe520dbafd72f5, 0x3ff96eae44cf4747, 0x40061947c21e23fe),
    (0x467318ba7b3a4770, 0xbfffd92bcc800bfb, 0x32f648415c93e495),
    (0x4073700000000000, 0xc01199999999999a, 0x3da7a9ed749a1404),
    (0x4000000000000000, 0xc027334cf79f0776, 0x3f351bf7cd9f1f7a),
    (0x4024000000000000, 0x40222af11a7f46bc, 0x41d2135560d3df4e),
    (0x404472e6a50980f9, 0x4002ed0f873238ba, 0x40b9637ddcb13147),
    (0x3fd0535e45b7e890, 0x4003c4724d7be715, 0x3fa1822e73c04ee9),
    (0x336079e691150e56, 0x3fea611cde89759c, 0x35943f1faa7b348f),
    (0x3fed479fa118204d, 0x4029936e9d42dd4c, 0x3fd48c921e67525e),
];

fn check1(name: &str, f: fn(f64) -> f64, table: &[(u64, u64)]) {
    for &(x, want) in table {
        let x = f64::from_bits(x);
        assert_eq!(f(x).to_bits(), want, "{name}({x:e})");
    }
}

#[test]
fn the_one_argument_functions_are_pythons() {
    check1("sin", dmath::sin, &SIN);
    check1("cos", dmath::cos, &COS);
    check1("tan", dmath::tan, &TAN);
    check1("asin", dmath::asin, &ASIN);
    check1("acos", dmath::acos, &ACOS);
    check1("atan", dmath::atan, &ATAN);
    check1("exp", dmath::exp, &EXP);
    check1("ln", dmath::ln, &LN);
    check1("log10", dmath::log10, &LOG10);
    check1("log2", dmath::log2, &LOG2);
    check1("sinh", dmath::sinh, &SINH);
}

#[test]
fn the_two_argument_functions_are_pythons() {
    for (y, x, want) in ATAN2 {
        let (y, x) = (f64::from_bits(y), f64::from_bits(x));
        assert_eq!(dmath::atan2(y, x).to_bits(), want, "atan2({y:e}, {x:e})");
    }
    for (x, y, want) in POW {
        let (x, y) = (f64::from_bits(x), f64::from_bits(y));
        assert_eq!(dmath::pow(x, y).to_bits(), want, "{x:e} ** {y:e}");
    }
}

#[test]
fn numpys_tanh_is_not_the_math_modules() {
    let mut differ = 0;
    for (x, math, numpy) in TANH {
        let x = f64::from_bits(x);
        assert_eq!(dmath::tanh(x).to_bits(), math, "math.tanh({x:e})");
        assert_eq!(dmath::np_tanh(x).to_bits(), numpy, "np.tanh({x:e})");
        differ += usize::from(math != numpy);
    }
    assert!(differ >= 14, "the table must keep arguments where the two differ");
    assert_eq!(dmath::np_tanh(0.0).to_bits(), 0);
    assert_eq!(dmath::np_tanh(-0.0).to_bits(), (-0.0f64).to_bits());
    assert_eq!(dmath::np_tanh(f64::INFINITY), 1.0);
    assert_eq!(dmath::np_tanh(-1e300), -1.0);
    assert!(dmath::np_tanh(f64::NAN).is_nan());
}

/// The values earlier rulings worked round with tables of Python's bits (M3-R20, R22, R27, R31) and the one
/// that started this audit: each is now simply what `dmath` computes.
#[test]
fn the_values_the_tables_were_for() {
    // 6 ** 0.6 (ORGAN's chord of six)
    assert_eq!(dmath::pow(6.0, 0.6).to_bits(), 0x4007_70f5_a7ed_d25b);
    // math.atan2(300.0, 360.0) (the title card's divider)
    assert_eq!(dmath::atan2(300.0, 360.0).to_bits(), 0x3fe6_3b4b_c3f1_6a8a);
    // math.sin / math.cos of math.radians(342) (the sprite camera)
    let a = f64::from_bits(0x4017_e048_5cda_5e0a);
    assert_eq!(dmath::sin(a).to_bits(), 0xbfd3_c6ef_372f_e954);
    assert_eq!(dmath::cos(a).to_bits(), 0x3fee_6f0e_1344_54ff);
}

/// A Python float squared with `**` is the C runtime's `pow(x, 2.0)`, not `x * x`: one step apart for about 1
/// argument in 2,000. (numpy squares an ARRAY `a ** 2` as `a * a`; a numpy scalar goes the way of the float.)
#[test]
fn a_float_squared_is_pow_not_a_product() {
    // (x, x ** 2, x * x) in Python
    for (x, pow, product) in [
        (
            0x3ff5_814e_8be8_3a3eu64,
            0x3ffc_e783_3353_326eu64,
            0x3ffc_e783_3353_326du64,
        ),
        (0x4001_4b8b_8042_1b48, 0x4012_b1ed_211f_2e3a, 0x4012_b1ed_211f_2e39),
        (0x3fff_b878_aa4c_40d2, 0x400f_7191_3761_2b46, 0x400f_7191_3761_2b47),
    ] {
        let x = f64::from_bits(x);
        assert_eq!(dmath::pow(x, 2.0).to_bits(), pow);
        assert_eq!((x * x).to_bits(), product);
    }
}

/// Where Python raises instead of answering, `dmath` gives the C result; the ends of each range behave.
#[test]
fn edges() {
    assert_eq!(dmath::sin(0.0).to_bits(), 0);
    assert_eq!(dmath::sin(-0.0).to_bits(), (-0.0f64).to_bits());
    assert_eq!(dmath::cos(0.0), 1.0);
    assert!(
        dmath::sin(f64::INFINITY).is_nan() && dmath::cos(f64::NAN).is_nan() && dmath::tan(f64::NEG_INFINITY).is_nan()
    );
    assert!(dmath::asin(1.5).is_nan() && dmath::acos(-1.5).is_nan());
    assert_eq!(dmath::acos(1.0), 0.0);
    assert_eq!(dmath::acos(-1.0), dmath::PI);
    assert_eq!(dmath::asin(1.0), dmath::PI / 2.0);
    assert_eq!(dmath::atan(f64::INFINITY), dmath::PI / 2.0);
    assert_eq!(dmath::atan2(0.0, -1.0), dmath::PI);
    assert_eq!(dmath::atan2(-0.0, -1.0), -dmath::PI);
    assert_eq!(dmath::atan2(0.0, 1.0).to_bits(), 0);
    assert_eq!(dmath::atan2(1.0, 0.0), dmath::PI / 2.0);
    assert_eq!(dmath::atan2(0.0, 0.0), 0.0);
    assert_eq!(dmath::exp(0.0), 1.0);
    assert_eq!(dmath::exp(1000.0), f64::INFINITY);
    assert_eq!(dmath::exp(-1000.0), 0.0);
    assert_eq!(dmath::exp(f64::NEG_INFINITY), 0.0);
    assert_eq!(dmath::ln(1.0), 0.0);
    assert_eq!(dmath::ln(0.0), f64::NEG_INFINITY);
    assert!(dmath::ln(-1.0).is_nan() && dmath::log10(-1.0).is_nan() && dmath::log2(-1.0).is_nan());
    assert_eq!(dmath::log10(1000.0), 3.0);
    assert_eq!(dmath::log2(8.0), 3.0);
    assert_eq!(dmath::ln(f64::INFINITY), f64::INFINITY);
    assert_eq!(dmath::pow(2.0, 10.0), 1024.0);
    assert_eq!(dmath::pow(0.0, 0.0), 1.0);
    assert_eq!(dmath::pow(-8.0, 3.0), -512.0);
    assert_eq!(dmath::pow(0.0, -1.0), f64::INFINITY);
    assert!(dmath::pow(-8.0, 0.5).is_nan());
    assert_eq!(dmath::pow(7.25, 1.0), 7.25);
    assert_eq!(dmath::pow(10.0, 400.0), f64::INFINITY);
    assert_eq!(dmath::pow(10.0, -400.0), 0.0);
    assert_eq!(dmath::tanh(40.0), 1.0);
    assert_eq!(dmath::sinh(0.0), 0.0);
    assert_eq!(dmath::sinh(1000.0), f64::INFINITY);
}
