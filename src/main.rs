pub mod constants;
#[cfg(test)]
pub mod test;

use std::{
    collections::HashMap,
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread,
};

use fs_extra::{copy_items, dir::CopyOptions, remove_items};
use log::{info, warn};
use notify::{Event, RecursiveMode, Watcher};
use regex::Regex;
use serde::{Deserialize, Serialize};

struct FileGirl {
    config: Config,
    file_hash_map: Arc<Mutex<HashMap<String, HashMap<String, Option<String>>>>>,
}
impl FileGirl {
    fn new(config: Config) -> Self {
        Self {
            config,
            file_hash_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    fn guard(self: Arc<Self>) -> Result<(), Box<dyn Error>> {
        let protected_dirs = self.config.protected_dirs.clone();
        for dir in protected_dirs {
            let self_clone = Arc::clone(&self);
            thread::spawn(move || {
                let mut file_hash_map = self_clone.file_hash_map.lock().unwrap();
                file_hash_map.insert(dir.clone(), FileGirl::make_file_hash_map(&dir).unwrap());
                drop(file_hash_map);
                info!("备份目录 {}", self_clone.config.backup_dir);
                let mut options = CopyOptions::new();
                options.overwrite = true;
                if Path::new(&self_clone.config.backup_dir).is_dir() == false {
                    fs::create_dir_all(&self_clone.config.backup_dir).unwrap();
                }
                let _ = fs_extra::dir::copy(&dir, &self_clone.config.backup_dir, &options);
                info!("监控 {}", dir);
                let (tx, rx) = mpsc::channel();
                let mut watcher = notify::RecommendedWatcher::new(
                    move |res| {
                        tx.send(res).unwrap();
                    },
                    notify::Config::default(),
                )
                .unwrap();
                let watching_path = Path::new(&dir);
                // todo error dealing
                match watcher.watch(watching_path, RecursiveMode::Recursive) {
                    Ok(_) => {
                        while let Ok(res) = rx.recv() {
                            match res {
                                Ok(event) => match self_clone.handle(event, dir.clone()) {
                                    Ok(_) => {}
                                    Err(e) => println!("handle error: {:?}", e),
                                },
                                Err(e) => println!("watch error: {:?}", e),
                            }
                        }
                    }
                    Err(e) => println!("watching directory {} error: {:?}", dir, e),
                }
            });
        }
        thread::park();
        Ok(())
    }

    fn handle(&self, e: Event, protected_dir: String) -> Result<(), Box<dyn Error>> {
        let filename = e.paths.first().unwrap().to_str().unwrap();

        for name in &self.config.white_names {
            match Regex::new(name) {
                Ok(re) => {
                    if re.is_match(Path::new(filename).file_name().unwrap().to_str().unwrap()) {
                        return Ok(());
                    }
                }
                Err(e) => return Err(Box::new(e)),
            }
        }

        let mut options = CopyOptions::new();
        options.overwrite = true;
        let event_paths = vec![filename.to_string()];

        let relative_path = Path::new(filename)
            .strip_prefix(Path::new(&protected_dir).parent().unwrap())
            .unwrap();

        let mut backup_path = PathBuf::from(&self.config.backup_dir);
        backup_path.push(relative_path);
        let backup_paths = vec![backup_path.to_str().unwrap().to_string()];
        /*
        create: 检查有没有在map里，如果没有就删除

        modify：检查有没有在map里，如果在，检查它的hash和map里的hash一不一样（不管是file还是dir），不一样就保存并写回

        remove：检查有没有在map里，如果在就写回
         */

        let binding = self.file_hash_map.lock().unwrap();

        let file_hash_map = binding.get(protected_dir.as_str()).unwrap();
        match e.kind {
            notify::EventKind::Create(_) => match file_hash_map.get(filename) {
                Some(_) => {}
                None => {
                    warn!("检测到创建 {}", filename);
                    let _ = remove_items(&event_paths);
                }
            },
            notify::EventKind::Modify(_) => {
                if let Some(backup_hash) = file_hash_map.get(filename) {
                    if FileGirl::calc_hash(filename).unwrap() != *backup_hash {
                        warn!("检测到修改 {}", filename);
                        let _ = copy_items(&event_paths, &self.config.backup_dir, &options);
                        // rollback
                        let _ = copy_items(&backup_paths, &protected_dir, &options);
                    }
                }
            }
            notify::EventKind::Remove(_) => {
                if let Some(_) = file_hash_map.get(filename) {
                    warn!("检测到删除 {}", filename);
                    // rollback
                    let _ = copy_items(
                        &backup_paths,
                        &Path::new(filename).parent().unwrap(),
                        &options,
                    );
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn calc_hash(path: &str) -> Result<Option<String>, Box<dyn Error>> {
        let file_path = Path::new(path);
        match fs::read(file_path) {
            Ok(content) => Ok(Some(format!("{:x}", md5::compute(content)))),
            Err(_) => {
                return Ok(None);
            }
        }
    }
    fn make_file_hash_map(path: &str) -> Result<HashMap<String, Option<String>>, Box<dyn Error>> {
        let mut file_hash_map = HashMap::new();
        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let key = path.to_string_lossy().to_string();
            if path.is_file() {
                if let Ok(content) = fs::read(path) {
                    let value = Some(format!("{:x}", md5::compute(content)));
                    file_hash_map.insert(key, value);
                }
            } else {
                let value = None;
                file_hash_map.insert(key, value);
            }
        }
        Ok(file_hash_map)
    }
}

#[derive(Serialize, Deserialize)]
struct Config {
    protected_dirs: Vec<String>,
    backup_dir: String,
    white_names: Vec<String>,
}
fn main() {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "init" {
        info!("创建config.yml");
        fs::write("./config.yml", constants::DEFAULT_CONFIG_YML).unwrap();
    } else if args.len() == 2 && args[1] == "run" {
        let config_str = fs::read_to_string(Path::new("./config.yml")).unwrap();
        info!("加载配置文件 ./config.yml");
        let config: Config = serde_yaml::from_str(&config_str).unwrap();
        let file_girl = Arc::new(FileGirl::new(config));
        file_girl.guard().unwrap();
    } else if args.len() == 4 && args[1] == "--config" && args[3] == "run" {
        let config_str = fs::read_to_string(Path::new(&args[2])).unwrap();
        info!("加载配置文件 {}", &args[2]);
        let config: Config = serde_yaml::from_str(&config_str).unwrap();
        let file_girl = Arc::new(FileGirl::new(config));
        file_girl.guard().unwrap();
    } else {
        println!("{}", constants::HELP);
    }
}
