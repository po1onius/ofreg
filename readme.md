# ofreg
## about
基于eBPF的Linux文件打开事件监控服务，记录目标目录下所有文件的打开事件，包含的信息有
|  被打开的文件的具体路径   | 打开文件的命令  | 打开文件的时间 |
|  ----  | ----  | ----  |
## usage
构建
```shell
cargo build --all
```
构建产物有两个命令
* 服务
```shell
ofregd config.toml
```
```toml
# config.toml说明
# 目标目录，监听哪个目录下的文件，如果是"/"就表示监听整个系统的文件
target_dir = "/home/user"

# 忽略的路径，指定哪些路径不被记录，防止一些文件打开过于频繁
ignore_path = ["/var/db/ofreg/", "/var/cache/fontconfig/"]

# 忽略的命令，指定哪些命令不被记录，防止一些命令打开文件过于频繁
ignore_cmd = ["fish"]
```


* cli
```shell
ofreg -c git #查看git命令打开过的文件
ofreg -f /home/user/.bashrc # 查看该文件被打开过的记录
```