# filegirl

基于[notify-rs](https://github.com/notify-rs/notify)实现的文件监控系统，支持自动备份、自动回滚、修改的文件自动记录。

用途：可用于check不严的AWD竞赛。

notify-rs可能不支持的场景参考[官方文档](https://docs.rs/notify/latest/notify/)，例如WSL的/mnt。


## 用法

**请务必在配置文件填写完整路径名**

```
filegirl init # 在当前目录下生成config.yml
filegirl --config <config.yml> run (Default: ./config.yml)
```

config.yml

```yml
protected_dirs: # 被监控的文件夹
    - D:\tmp\test\framework
backup_dir: D:\tmp\test\backup
white_names: # 白名单文件（支持正则）
    - filegirl
    - .*\.dist
```

## 功能

被监控的文件夹发生的事件和对应的处理逻辑如下

| 事件   | 行为                                                         |
| ------ | ------------------------------------------------------------ |
| create | 检查有没有在map里，如果没有就删除                            |
| modify | 检查有没有在map里，如果在，检查map里的hash，不一样就保存到backup_dir并rollback |
| remove | 检查有没有在map里，如果在就rollback                          |

map在程序里的定义如下：

 `Arc<Mutex<HashMap<String, HashMap<String, Option<String>>>>>` 

第一层map的key是文件夹名称

第二层map的key是文件路径，value是其md5，如果key是文件夹则value是None

