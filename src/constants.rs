pub const DEFAULT_CONFIG_YML: &str = "protected_dirs: # 被监控的文件夹
    - /var/www/html/
backup_dir: /tmp/backup
white_names: # 白名单文件（支持正则）
    - filegirl
";
pub const HELP: &str = "Usage:
    filegirl init
    filegirl --config <config.yml> run (Default: ./config.yml)";
