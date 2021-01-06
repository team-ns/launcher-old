<a name="0.0.1"></a>
## 0.0.1 NSLauncher (2021-01-06)


#### Bug Fixes

*   rollback tokio version to 0.2 ([cfb67e4a](cfb67e4a))
* **client:**
  *  fixing settings field on update ([8af3a827](8af3a827))
  *  fixing update configuration file ([4d3b69ba](4d3b69ba))
  *  change dependency target ([7815babf](7815babf))
  *  fix get os type on linux ([5a37ae04](5a37ae04))
  *  set path to profile ([7363d923](7363d923))
  *  set current directory and change JRE path ([528aa7c3](528aa7c3))

#### Features

* **api:**
  *  use path instead string in remote directory ([6c21d44f](6c21d44f))
  *  add server info in profile ([5ec62bf7](5ec62bf7))
  *  add fields for watcher file update ([473029fb](473029fb))
  *  add RemoteFile for download and rehash ([6cb10ae9](6cb10ae9))
* **client:**
  *  remove file server field from config ([5234ede5](5234ede5))
  *  change watcher event logic, add error handling for game start ([ebe68537](ebe68537))
  *  remove unknown files and exclude files for downloading ([09857c70](09857c70))
  *  adding server connection game argument ([1b146cff](1b146cff))
  *  remove file if exist before download ([49e8e02c](49e8e02c))
  *  error handling in jvm start ([38af106d](38af106d))
  *  set window subsystem ([299762ff](299762ff))
  *  add icon for windows exe, set window title from config, obfuscation configuration ([cdfcd11e](cdfcd11e))
  *  create watcher and integrate validation with RemoteFile ([61b52d22](61b52d22))
  *  use url and file limit in downloader system ([450b2475](450b2475))
  *  remove config field from client ([c726c965](c726c965))
  *  processing settings messages from runtime, save user info and send profile list ([a6c084ad](a6c084ad))
  *  function for get profile list and encrypted password ([9b0a24cd](9b0a24cd))
  *  update config format and create client settings ([3b8d1dda](3b8d1dda))
  *  dynamic detection os arch ([181541b9](181541b9))
  *  refactoring webview runtime, use once_cell for global client and native method for server join ([32b604dd](32b604dd))
* **server:**
  *  use anyhow in message error handle, save session info and remove http join request ([affa2cee](affa2cee))
  *  add api key for json auth and initialize config values after serialize ([ac2730b8](ac2730b8))
  *  update rehash logic, store url for file downloading ([8aaa39bc](8aaa39bc))
  *  add new fields in config for building and rehash ([a37b0384](a37b0384))
  *  implementation join server message ([4cc0f5f7](4cc0f5f7))
  *  write logger configuration for release build ([f428df19](f428df19))
  *  use log4rs instead env_logger ([ce9873c3](ce9873c3))
  *  bundling launcher and launcherapi in launchserver ([6b941c96](6b941c96))



