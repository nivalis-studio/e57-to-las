# Changelog

All notable changes to this project will be documented in this file.

## [0.4.7](https://github.com/wildweb-io/e57_to_las/compare/v0.4.6..v0.4.7) - 2024-06-13

### ⛰️  Features

- Bump dependencies - ([906f187](https://github.com/wildweb-io/e57_to_las/commit/906f187eb9d8d05f589c97d67463069db60937a4))

### ⚙️ Miscellaneous Tasks

- Update README - ([5430f44](https://github.com/wildweb-io/e57_to_las/commit/5430f44df4d5d7d58cf2957e6be44024d5e25d4f))

## [0.4.6](https://github.com/wildweb-io/e57_to_las/compare/v0.4.5..v0.4.6) - 2024-04-17

### ⛰️  Features

- Bump dependencies - ([c7c13b7](https://github.com/wildweb-io/e57_to_las/commit/c7c13b71b90995c9b23196eea44472d383b459cd))

### ⚙️ Miscellaneous Tasks

- Update CHANGELOG.md - ([f2874b3](https://github.com/wildweb-io/e57_to_las/commit/f2874b3cf795d83d7b95eb792c7850912bf46422))
- Bump version number - ([1ecb546](https://github.com/wildweb-io/e57_to_las/commit/1ecb5461e3bd74e13c73b2c3c472521dd45fa9d5))

### Deps

- Bump e57 crate version - ([9a9843d](https://github.com/wildweb-io/e57_to_las/commit/9a9843d8774c9b6cdd0b479d01919e4c45b91f42))

## [0.4.5](https://github.com/wildweb-io/e57_to_las/compare/v0.4.4..v0.4.5) - 2023-10-27

### ⚙️ Miscellaneous Tasks

- Remove stations feature from release workflow - ([6217b09](https://github.com/wildweb-io/e57_to_las/commit/6217b09e54e8e4242e4a7e692dc5a0cb99f1e6f0))

### Deps

- Bump e57rs - ([24463d3](https://github.com/wildweb-io/e57_to_las/commit/24463d3f6a81c7e15f4087ad6a2908ae2b2c499c))

## [0.4.4](https://github.com/wildweb-io/e57_to_las/compare/v0.4.3..v0.4.4) - 2023-09-29

### ⚙️ Miscellaneous Tasks

- Fix i32 to f64 conversion - ([85f1654](https://github.com/wildweb-io/e57_to_las/commit/85f1654a015ff78c520654bde18bb51ffbdaa34e))
- Update CHANGELOG - ([0e5ea75](https://github.com/wildweb-io/e57_to_las/commit/0e5ea7585955db7734a333bfcd42271b8308f863))

## [0.4.3](https://github.com/wildweb-io/e57_to_las/compare/v0.4.1..0.4.2) - 2023-09-27

### Bug Fixes

- fix inverse transform error on points with big coordinates by playing with scale

## [0.4.2](https://github.com/wildweb-io/e57_to_las/compare/v0.4.1..0.4.2) - 2023-09-05

## [0.4.1](https://github.com/wildweb-io/e57_to_las/compare/v0.4.0..0.4.1) - 2023-09-05

### ⛰️ Features

- Bump e57 crate version - ([8eac110](https://github.com/wildweb-io/e57_to_las/commit/8eac1108703f151c431a926941ebf127ecc1c6f6))

## [0.4.0](https://github.com/wildweb-io/e57_to_las/compare/v0.3.1..0.4.0) - 2023-08-25

### ⛰️ Features

- Make stations output file optional - ([aa620c9](https://github.com/wildweb-io/e57_to_las/commit/aa620c9fd689491549e72903b6cae3bb5a53c58e))

### ⚙️ Miscellaneous Tasks

- _(release)_ Bump to v0.4.0 - ([0b1215a](https://github.com/wildweb-io/e57_to_las/commit/0b1215a6d318b165bb1b8f98934be6f5103448e5))
- Go back to hashmap of stations - ([d0128ae](https://github.com/wildweb-io/e57_to_las/commit/d0128ae421d66f7ad24fa86ac6401f93abea374e))
- Cleanup - ([f77be3f](https://github.com/wildweb-io/e57_to_las/commit/f77be3fcd8402adbee0adbcca21ba0cc6dfa7200))
- Move to vec stations and refactor - ([53ad789](https://github.com/wildweb-io/e57_to_las/commit/53ad78937a722898effe5451588905eef11436bd))

## [0.3.1](https://github.com/wildweb-io/e57_to_las/compare/v0.3.0..0.3.1) - 2023-08-23

### ⛰️ Features

- Improve station points calculation - ([aa58476](https://github.com/wildweb-io/e57_to_las/commit/aa58476380ce8e82734dc7bc81ec7c5e4e8adc7b))

### 📚 Documentation

- Add docs.rs to README - ([9c0fd01](https://github.com/wildweb-io/e57_to_las/commit/9c0fd0115f31d427d97dd577cc6fc83daba526dc))
- Update README - ([95a892f](https://github.com/wildweb-io/e57_to_las/commit/95a892ff17ba4619a9a9b835d7d44a094f779e8a))
- Update changelog for v0.3 - ([82347af](https://github.com/wildweb-io/e57_to_las/commit/82347af9ceac59748f30600411cc583bc87f37a5))

## [0.3.0](https://github.com/wildweb-io/e57_to_las/compare/v0.2.1..0.3.0) - 2023-08-22

### ⛰️ Features

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

- _(release)_ V0.2.1 - ([3df9268](https://github.com/wildweb-io/e57_to_las/commit/3df9268e0e59a81ddc815c40176ff0925f80bc50))
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

### ⛰️ Features

- Improve color and intensity calculations - ([aa6f164](https://github.com/wildweb-io/e57_to_las/commit/aa6f164739aeece57d4d2a5371786d16b65a5fff))

### 📚 Documentation

- Add badges to README - ([242950b](https://github.com/wildweb-io/e57_to_las/commit/242950bf6216853a00953a4d5913c76ac848ed2a))

### ⚙️ Miscellaneous Tasks

- _(release)_ V0.2.1 - ([870565c](https://github.com/wildweb-io/e57_to_las/commit/870565c69e59fe4aa644c4add3aa77c86a9ae8c4))
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

- Add LICENSE - ([a55fa40](https://github.com/wildweb-io/e57_t
