# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0](https://github.com/wildweb-io/e57-to-las-rs/compare/v0.1.1..0.2.0) - 2023-08-21

### ⛰️ Features

- Add README.md - ([075c527](https://github.com/wildweb-io/e57-to-las-rs/commit/075c527b9490e4d3ddd80431b2c93ec487cfa597))
- Add xml header read - ([05a723e](https://github.com/wildweb-io/e57-to-las-rs/commit/05a723ee626afb9a2c0136a0f2f53198a8850991))
- Add replace for uuid - ([c62a56b](https://github.com/wildweb-io/e57-to-las-rs/commit/c62a56bbaf581fdbda36c3b8fd9f183562f9bc50))

### 🚜 Refactor

- Improve error logging - ([238ffbf](https://github.com/wildweb-io/e57-to-las-rs/commit/238ffbfbdfd59d194c87ac41bae4c882686f3572))

### 📚 Documentation

- Add git cliff to generate changelog - ([8d6e59e](https://github.com/wildweb-io/e57-to-las-rs/commit/8d6e59ef9f8830f0bdf2cf8376816aaa9b2a3227))

### ⚙️ Miscellaneous Tasks

- Add a way to specifiy the number of threads wanted to avoid full cpu usage - ([81ae675](https://github.com/wildweb-io/e57-to-las-rs/commit/81ae6751dc4423096a787abc391375dce7aac1ee))
- Cleanup xml header testing - ([1e3a8a4](https://github.com/wildweb-io/e57-to-las-rs/commit/1e3a8a4f2275bb98f1214665d7aeaff4f6f70066))
- Remove uncessary point rotation and translation - ([20dbf80](https://github.com/wildweb-io/e57-to-las-rs/commit/20dbf800593db827fbb7e02d5bcde75b21d96d96))

## [0.1.1](https://github.com/wildweb-io/e57-to-las-rs/compare/v0.1.0..v0.1.1) - 2023-08-16

### ⚙️ Miscellaneous Tasks

- Add metadata - ([0160da8](https://github.com/wildweb-io/e57-to-las-rs/commit/0160da8987e29325d0f99d902e56cad56c726f75))

## [0.1.0](https://github.com/wildweb-io/e57-to-las-rs/compare/v0.0.6..v0.1.0) - 2023-08-16

### 🚜 Refactor

- Remove unused mutex and pointcloud center slow processing - ([3e1bb64](https://github.com/wildweb-io/e57-to-las-rs/commit/3e1bb64210094bf019c84e2b4ae5fa41ed8a8951))

### ⚙️ Miscellaneous Tasks

- Add LICENSE - ([a55fa40](https://github.com/wildweb-io/e57-to-las-rs/commit/a55fa4084d01704fb67c70d24244739d59c5e7a6))
- Update output hierarchy to be uuid/las/\*.las - ([a9675c1](https://github.com/wildweb-io/e57-to-las-rs/commit/a9675c184407813777eb869fe34d98e2dc11e1b9))
- Calculate pointclouds centers and write in json - ([245340b](https://github.com/wildweb-io/e57-to-las-rs/commit/245340b9539776150262f08a6816335ba84b71a7))

## [0.0.6](https://github.com/wildweb-io/e57-to-las-rs/compare/v0.0.5..v0.0.6) - 2023-08-16

### ⛰️ Features

- ParallelIterator to process each point cloud - ([7eda28f](https://github.com/wildweb-io/e57-to-las-rs/commit/7eda28f0bd0077230a15346a8689c067eac81d4b))

### ⚙️ Miscellaneous Tasks

- Skip invalid point and refactor - ([f1a1ad1](https://github.com/wildweb-io/e57-to-las-rs/commit/f1a1ad1f5de45cfb4113eb29e4f1125b48e51e97))
- Bump to e57 0.6.0 - ([8cff01a](https://github.com/wildweb-io/e57-to-las-rs/commit/8cff01a3c25a1850a4bdfe4aee0953b51a01f3dd))
- Small refactor - ([1d14a99](https://github.com/wildweb-io/e57-to-las-rs/commit/1d14a99eb0b085ea36646ec038710b2b0e784a64))
- Inside pointcloud multi-threading - ([48e046a](https://github.com/wildweb-io/e57-to-las-rs/commit/48e046aac81c2b240b484965ea8a45a2dbf094ce))

## [0.0.5](https://github.com/wildweb-io/e57-to-las-rs/compare/v0.0.4..v0.0.5) - 2023-08-11

### ⚙️ Miscellaneous Tasks

- Remove duplicate gh workflows - ([dd1a88f](https://github.com/wildweb-io/e57-to-las-rs/commit/dd1a88f6d07022ac9d4ecb6c0632ec5ba49a457f))

## [0.0.4](https://github.com/wildweb-io/e57-to-las-rs/compare/v0.0.3..v0.0.4) - 2023-08-11

### 🐛 Bug Fixes

- Add default rgb to point without color - ([a154509](https://github.com/wildweb-io/e57-to-las-rs/commit/a15450956e34e277f492c3eb23d0fa6f55160bc3))

### 🚜 Refactor

- Make progress bar optional (-P or --progress) - ([90d2765](https://github.com/wildweb-io/e57-to-las-rs/commit/90d27658d6f64a24e5970cb95b41e34463020c7b))
- Create output sub dir to store generated las files - ([5b1e47c](https://github.com/wildweb-io/e57-to-las-rs/commit/5b1e47c8ed78c7362b8eb8b870a8df9db4f43c4c))
- Cleanup and renaming - ([7534c0e](https://github.com/wildweb-io/e57-to-las-rs/commit/7534c0e5dec64bf25e229bea815cc1eba3f67654))

### ⚙️ Miscellaneous Tasks

- Add github action to build and release on tag push - ([f02a118](https://github.com/wildweb-io/e57-to-las-rs/commit/f02a1186694e2cbc32c6c13147611473321438ea))
- Remove dbg! - ([e430ba9](https://github.com/wildweb-io/e57-to-las-rs/commit/e430ba985f7e084e7c05193eadf83e754ebf18f8))
- Add output cli arg and use it in las path constructor fn - ([a6ec55c](https://github.com/wildweb-io/e57-to-las-rs/commit/a6ec55ced5c46d464044a4b641730e39dca5fc91))
- Add progress bar to each pointcloud convertion - ([ce01eb0](https://github.com/wildweb-io/e57-to-las-rs/commit/ce01eb0340f4d5a8ac78b0ab3c4536deb67ceb96))
- Gitignore las files - ([ccbe34e](https://github.com/wildweb-io/e57-to-las-rs/commit/ccbe34e469a557c34526cc9c668da8d7a49d0f0e))
- Update las writer header to include colors - ([7dd0655](https://github.com/wildweb-io/e57-to-las-rs/commit/7dd06554674c4321deff4dcb9ad3ccc323b53187))
- Fix f32 to u16 rgb conversion - ([f0010e4](https://github.com/wildweb-io/e57-to-las-rs/commit/f0010e484391fdf2b3605d8d7599f28263a62500))

## [0.0.3](https://github.com/wildweb-io/e57-to-las-rs/compare/v0.0.2..v0.0.3) - 2023-08-09

## [0.0.2](https://github.com/wildweb-io/e57-to-las-rs/compare/v0.0.1..v0.0.2) - 2023-08-09

### ⛰️ Features

- Update binary upload - ([5262560](https://github.com/wildweb-io/e57-to-las-rs/commit/5262560dc3178efc86c6edde9940d7c297396ecc))

## [0.0.1] - 2023-08-09

### ⛰️ Features

- Add default transform - ([523feb5](https://github.com/wildweb-io/e57-to-las-rs/commit/523feb52353a26d161e429673aa3ab3a1de9a387))
- Basic cli to convert e57 to las - ([3cc2ba4](https://github.com/wildweb-io/e57-to-las-rs/commit/3cc2ba4699c7f5e85f44e544587ea010acd46af4))

### 🚜 Refactor

- Refactor
- ([3324ef7](https://github.com/wildweb-io/e57-to-las-rs/commit/3324ef703855a656735361b3ac18430f8b31e1c2))
- Prefer early return - ([203d76b](https://github.com/wildweb-io/e57-to-las-rs/commit/203d76b03cd114a952acb92344916752ed2fe088))

### ⚙️ Miscellaneous Tasks

- Add basic gh action - ([ee49c75](https://github.com/wildweb-io/e57-to-las-rs/commit/ee49c75edae7307b5dfe0a019a367fcf84b0fe79))
- Cleanup - ([4f68c61](https://github.com/wildweb-io/e57-to-las-rs/commit/4f68c61927c9cfa9d94987e85bf4d907a9b5db86))
- Cleanup - ([26bd3dc](https://github.com/wildweb-io/e57-to-las-rs/commit/26bd3dccecff2649a6582b3c077da08058233ecd))
- Add gitignore - ([9f65cd5](https://github.com/wildweb-io/e57-to-las-rs/commit/9f65cd53345c9f9a4a01f7fd447139488137e1bd))
