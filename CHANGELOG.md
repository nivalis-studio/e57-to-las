# Changelog

All notable changes to this project will be documented in this file.

## [0.3.1](https://github.com/wildweb-io/e57_to_las/compare/v0.3.0..0.3.1) - 2023-08-23

### ⛰️  Features

- Improve station points calculation - ([aa58476](https://github.com/wildweb-io/e57_to_las/commit/aa58476380ce8e82734dc7bc81ec7c5e4e8adc7b))

### 📚 Documentation

- Add docs.rs to README - ([9c0fd01](https://github.com/wildweb-io/e57_to_las/commit/9c0fd0115f31d427d97dd577cc6fc83daba526dc))
- Update README - ([95a892f](https://github.com/wildweb-io/e57_to_las/commit/95a892ff17ba4619a9a9b835d7d44a094f779e8a))
- Update changelog for v0.3 - ([82347af](https://github.com/wildweb-io/e57_to_las/commit/82347af9ceac59748f30600411cc583bc87f37a5))

## [0.3.0](https://github.com/wildweb-io/e57_to_las/compare/v0.2.1..0.3.0) - 2023-08-22

### ⛰️  Features

- Extract `get_las_writer` fn - ([2bfe276](https://github.com/wildweb-io/e57_to_las/commit/2bfe2764c005327a0b58bf846f1943605607fe89))
- Extract `create_station_file` function - ([24378fe](https://github.com/wildweb-io/e57_to_las/commit/24378fe805a594b4eff0e606c761af67478c51d3))
- Add convert file fn - ([ccf390e](https://github.com/wildweb-io/e57_to_las/commit/ccf390ef1b717d37135f05e1122f0f77ccf096e6))
- Create lib - ([2da027f](https://github.com/wildweb-io/e57_to_las/commit/2da027fc977cbc26ac81d73e2f5fea8852096470))

### 🚜 Refactor

- Clean up lib exports - ([a88686a](https://github.com/wildweb-io/e57_to_las/commit/a88686acf0589d8cae14f46b6ff4044a835b76c5))
- Replace matches with error propagation - ([711fd2d](https://github.com/wildweb-io/e57_to_las/commit/711fd2dd7314178b17d64a2e465e2b97e9891469))
- Adopt a more modular approach - ([108ee47](https://github.com/wildweb-io/e57_to_las/commit/108ee47d4cd95a477d051cbcec56e49d70463b8b))

### 📚 Documentation

- Add rustdoc for the exported functions - ([227ff45](https://github.com/wildweb-io/e57_to_las/commit/227ff45f8bb6c17ec2a03638414d641fd20e2092))
- Update CHANGELOG - ([32e979e](https://github.com/wildweb-io/e57_to_las/commit/32e979e265f16f5915cadf0e171bcb42229a19ba))

### ⚙️ Miscellaneous Tasks

- *(release)* V0.2.1 - ([3df9268](https://github.com/wildweb-io/e57_to_las/commit/3df9268e0e59a81ddc815c40176ff0925f80bc50))
- Refactor point conversion in separate mod - ([d400f96](https://github.com/wildweb-io/e57_to_las/commit/d400f96d6366bb90be7d6db17df1d8a3c0bd611e))
- Remove potentially unused functions - ([950c663](https://github.com/wildweb-io/e57_to_las/commit/950c6631e56764c9f3cfd4333051e80b085cff7a))
- Update point conversion to e57 version and example - ([a8b6a29](https://github.com/wildweb-io/e57_to_las/commit/a8b6a2951db114fe81eec1ea1d8ad9aa426c2311))
- Remove deprecated pointcloud reader method - ([364aa6b](https://github.com/wildweb-io/e57_to_las/commit/364aa6be98322196d33ddf31f94729faf17c8a70))
- Update sum_coordinates to check the validity of current coordinates - ([465c958](https://github.com/wildweb-io/e57_to_las/commit/465c95844b9bb87d48505db40ebe1df5515a5db5))
- Replace eventual invalid guid by random uuid - ([1b6f8f4](https://github.com/wildweb-io/e57_to_las/commit/1b6f8f428a97da07976e668647e5c4dcf8ac89b0))
- Refactor stations from vec to hashmap - ([101ec94](https://github.com/wildweb-io/e57_to_las/commit/101ec943438ef3c7b157d32055253c310e8ddb30))
- Move thread pool builder in convert_file mod - ([c1cb0e4](https://github.com/wildweb-io/e57_to_las/commit/c1cb0e4c2116268e569bafa59a3bcbc87279c5d5))
- Update README - ([55f8200](https://github.com/wildweb-io/e57_to_las/commit/55f82007a1449befe903d189658e9b27df6880c6))
- Cleanup unusued crates deps - ([9a242f3](https://github.com/wildweb-io/e57_to_las/commit/9a242f33146f5e4f78438357d00d60f750a2b002))
- Rename get_sum_coordinates - ([8fa134a](https://github.com/wildweb-io/e57_to_las/commit/8fa134a35b4784765c1efbb42c95243a1269459a))

### Deps

- Bump e57 to 0.8.0 - ([c17f576](https://github.com/wildweb-io/e57_to_las/commit/c17f57685b78d16da964a37ca993e62f9d585eb1))
- Add v4 feature to uuid crate - ([8d6afc9](https://github.com/wildweb-io/e57_to_las/commit/8d6afc9aa42c8291f475d1b68a6f0de74509d9cc))

## [0.2.1](https://github.com/wildweb-io/e57_to_las/compare/v0.2.0..v0.2.1) - 2023-08-22

### ⛰️  Features

- Improve color and intensity calculations - ([aa6f164](https://github.com/wildweb-io/e57_to_las/commit/aa6f164739aeece57d4d2a5371786d16b65a5fff))

### 📚 Documentation

- Add badges to README - ([242950b](https://github.com/wildweb-io/e57_to_las/commit/242950bf6216853a00953a4d5913c76ac848ed2a))

### ⚙️ Miscellaneous Tasks

- *(release)* V0.2.1 - ([870565c](https://github.com/wildweb-io/e57_to_las/commit/870565c69e59fe4aa644c4add3aa77c86a9ae8c4))
- Update repo and bin name - ([321a55a](https://github.com/wildweb-io/e57_to_las/commit/321a55a12543af35a930c7696fc6f03353aa55d4))
- Bump e57 crate to v0.7.0 - ([cb7f9e9](https://github.com/wildweb-io/e57_to_las/commit/cb7f9e98e51ad8794277c2b0785cc7a2d264a782))

## [0.2.0](https://github.com/wildweb-io/e57_to_las/compare/v0.1.1..0.2.0) - 2023-08-21

### ⛰️ Features

- Add README.md - ([075c527](https://github.com/wildweb-io/e57_to_las/commit/075c527b9490e4d3ddd80431b2c93ec487cfa597))
- Add xml header read - ([05a723e](https://github.com/wildweb-io/e57_to_las/commit/05a723ee626afb9a2c0136a0f2f53198a8850991))
- Add replace for uuid - ([c62a56b](https://github.com/wildweb-io/e57_to_las/commit/c62a56bbaf581fdbda36c3b8fd9f183562f9bc50))

### 🚜 Refactor

- Improve error logging - ([238ffbf](https://github.com/wildweb-io/e57_to_las/commit/238ffbfbdfd59d194c87ac41bae4c882686f3572))

### 📚 Documentation

- Add git cliff to generate changelog - ([8d6e59e](https://github.com/wildweb-io/e57_to_las/commit/8d6e59ef9f8830f0bdf2cf8376816aaa9b2a3227))

### ⚙️ Miscellaneous Tasks

- Add a way to specifiy the number of threads wanted to avoid full cpu usage - ([81ae675](https://github.com/wildweb-io/e57_to_las/commit/81ae6751dc4423096a787abc391375dce7aac1ee))
- Cleanup xml header testing - ([1e3a8a4](https://github.com/wildweb-io/e57_to_las/commit/1e3a8a4f2275bb98f1214665d7aeaff4f6f70066))
- Remove uncessary point rotation and translation - ([20dbf80](https://github.com/wildweb-io/e57_to_las/commit/20dbf800593db827fbb7e02d5bcde75b21d96d96))

## [0.1.1](https://github.com/wildweb-io/e57_to_las/compare/v0.1.0..v0.1.1) - 2023-08-16

### ⚙️ Miscellaneous Tasks

- Add metadata - ([0160da8](https://github.com/wildweb-io/e57_to_las/commit/0160da8987e29325d0f99d902e56cad56c726f75))

## [0.1.0](https://github.com/wildweb-io/e57_to_las/compare/v0.0.6..v0.1.0) - 2023-08-16

### 🚜 Refactor

- Remove unused mutex and pointcloud center slow processing - ([3e1bb64](https://github.com/wildweb-io/e57_to_las/commit/3e1bb64210094bf019c84e2b4ae5fa41ed8a8951))

### ⚙️ Miscellaneous Tasks

- Add LICENSE - ([a55fa40](https://github.com/wildweb-io/e57_to_las/commit/a55fa4084d01704fb67c70d24244739d59c5e7a6))
- Update output hierarchy to be uuid/las/\*.las - ([a9675c1](https://github.com/wildweb-io/e57_to_las/commit/a9675c184407813777eb869fe34d98e2dc11e1b9))
- Calculate pointclouds centers and write in json - ([245340b](https://github.com/wildweb-io/e57_to_las/commit/245340b9539776150262f08a6816335ba84b71a7))

## [0.0.6](https://github.com/wildweb-io/e57_to_las/compare/v0.0.5..v0.0.6) - 2023-08-16

### ⛰️ Features

- ParallelIterator to process each point cloud - ([7eda28f](https://github.com/wildweb-io/e57_to_las/commit/7eda28f0bd0077230a15346a8689c067eac81d4b))

### ⚙️ Miscellaneous Tasks

- Skip invalid point and refactor - ([f1a1ad1](https://github.com/wildweb-io/e57_to_las/commit/f1a1ad1f5de45cfb4113eb29e4f1125b48e51e97))
- Bump to e57 0.6.0 - ([8cff01a](https://github.com/wildweb-io/e57_to_las/commit/8cff01a3c25a1850a4bdfe4aee0953b51a01f3dd))
- Small refactor - ([1d14a99](https://github.com/wildweb-io/e57_to_las/commit/1d14a99eb0b085ea36646ec038710b2b0e784a64))
- Inside pointcloud multi-threading - ([48e046a](https://github.com/wildweb-io/e57_to_las/commit/48e046aac81c2b240b484965ea8a45a2dbf094ce))

## [0.0.5](https://github.com/wildweb-io/e57_to_las/compare/v0.0.4..v0.0.5) - 2023-08-11

### ⚙️ Miscellaneous Tasks

- Remove duplicate gh workflows - ([dd1a88f](https://github.com/wildweb-io/e57_to_las/commit/dd1a88f6d07022ac9d4ecb6c0632ec5ba49a457f))

## [0.0.4](https://github.com/wildweb-io/e57_to_las/compare/v0.0.3..v0.0.4) - 2023-08-11

### 🐛 Bug Fixes

- Add default rgb to point without color - ([a154509](https://github.com/wildweb-io/e57_to_las/commit/a15450956e34e277f492c3eb23d0fa6f55160bc3))

### 🚜 Refactor

- Make progress bar optional (-P or --progress) - ([90d2765](https://github.com/wildweb-io/e57_to_las/commit/90d27658d6f64a24e5970cb95b41e34463020c7b))
- Create output sub dir to store generated las files - ([5b1e47c](https://github.com/wildweb-io/e57_to_las/commit/5b1e47c8ed78c7362b8eb8b870a8df9db4f43c4c))
- Cleanup and renaming - ([7534c0e](https://github.com/wildweb-io/e57_to_las/commit/7534c0e5dec64bf25e229bea815cc1eba3f67654))

### ⚙️ Miscellaneous Tasks

- Add github action to build and release on tag push - ([f02a118](https://github.com/wildweb-io/e57_to_las/commit/f02a1186694e2cbc32c6c13147611473321438ea))
- Remove dbg! - ([e430ba9](https://github.com/wildweb-io/e57_to_las/commit/e430ba985f7e084e7c05193eadf83e754ebf18f8))
- Add output cli arg and use it in las path constructor fn - ([a6ec55c](https://github.com/wildweb-io/e57_to_las/commit/a6ec55ced5c46d464044a4b641730e39dca5fc91))
- Add progress bar to each pointcloud convertion - ([ce01eb0](https://github.com/wildweb-io/e57_to_las/commit/ce01eb0340f4d5a8ac78b0ab3c4536deb67ceb96))
- Gitignore las files - ([ccbe34e](https://github.com/wildweb-io/e57_to_las/commit/ccbe34e469a557c34526cc9c668da8d7a49d0f0e))
- Update las writer header to include colors - ([7dd0655](https://github.com/wildweb-io/e57_to_las/commit/7dd06554674c4321deff4dcb9ad3ccc323b53187))
- Fix f32 to u16 rgb conversion - ([f0010e4](https://github.com/wildweb-io/e57_to_las/commit/f0010e484391fdf2b3605d8d7599f28263a62500))

## [0.0.3](https://github.com/wildweb-io/e57_to_las/compare/v0.0.2..v0.0.3) - 2023-08-09

## [0.0.2](https://github.com/wildweb-io/e57_to_las/compare/v0.0.1..v0.0.2) - 2023-08-09

### ⛰️ Features

- Update binary upload - ([5262560](https://github.com/wildweb-io/e57_to_las/commit/5262560dc3178efc86c6edde9940d7c297396ecc))

## [0.0.1] - 2023-08-09

### ⛰️ Features

- Add default transform - ([523feb5](https://github.com/wildweb-io/e57_to_las/commit/523feb52353a26d161e429673aa3ab3a1de9a387))
- Basic cli to convert e57 to las - ([3cc2ba4](https://github.com/wildweb-io/e57_to_las/commit/3cc2ba4699c7f5e85f44e544587ea010acd46af4))

### 🚜 Refactor

- Refactor
- ([3324ef7](https://github.com/wildweb-io/e57_to_las/commit/3324ef703855a656735361b3ac18430f8b31e1c2))
- Prefer early return - ([203d76b](https://github.com/wildweb-io/e57_to_las/commit/203d76b03cd114a952acb92344916752ed2fe088))

### ⚙️ Miscellaneous Tasks

- Add basic gh action - ([ee49c75](https://github.com/wildweb-io/e57_to_las/commit/ee49c75edae7307b5dfe0a019a367fcf84b0fe79))
- Cleanup - ([4f68c61](https://github.com/wildweb-io/e57_to_las/commit/4f68c61927c9cfa9d94987e85bf4d907a9b5db86))
- Cleanup - ([26bd3dc](https://github.com/wildweb-io/e57_to_las/commit/26bd3dccecff2649a6582b3c077da08058233ecd))
- Add gitignore - ([9f65cd5](https://github.com/wildweb-io/e57_to_las/commit/9f65cd53345c9f9a4a01f7fd447139488137e1bd))
