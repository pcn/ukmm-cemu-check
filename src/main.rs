use serde_yaml::{Value, Mapping};
use std::path::{Path, PathBuf};
use std::fs;
use xml::reader::{EventReader, XmlEvent};

fn main() {
    let verifier = ConfigVerifier::new();
    std::process::exit(verifier.run());
}

struct ConfigVerifier {
    home: PathBuf,
    cemu_config: PathBuf,
    ukmm_config: PathBuf,
    errors: i32,
}

#[derive(Debug)]
struct CemuConfig {
    game_paths: Vec<String>,
}

#[derive(Debug)]
struct UkmConfig {
    host_path: Option<String>,
    content_dir: Option<String>,
    update_dir: Option<String>,
    aoc_dir: Option<String>,
    deploy_config: Option<DeployConfig>,
}

#[derive(Debug, Clone)]
struct DeployConfig {
    output: Option<String>,
    layout: Option<String>,
    method: Option<String>,
}

impl ConfigVerifier {
    fn new() -> Self {
        let home = home::home_dir().unwrap_or_else(|| PathBuf::from("."));
        ConfigVerifier {
            cemu_config: home.join(".config/Cemu/settings.xml"),
            ukmm_config: home.join(".config/ukmm/settings.yml"),
            home,
            errors: 0,
        }
    }

    fn load_cemu_config(&self) -> Result<CemuConfig, Box<dyn std::error::Error>> {
        if !self.cemu_config.exists() {
            eprintln!("❌ Cemu config not found: {}", self.cemu_config.display());
            std::process::exit(1);
        }

        let file = fs::File::open(&self.cemu_config)?;
        let parser = EventReader::new(file);

        let mut game_paths = Vec::new();
        let mut in_game_paths = false;
        let mut in_entry = false;
        let mut current_entry = String::new();

        for event in parser {
            match event? {
                XmlEvent::StartElement { name, .. } => {
                    if name.local_name == "GamePaths" {
                        in_game_paths = true;
                    } else if name.local_name == "Entry" && in_game_paths {
                        in_entry = true;
                    }
                }
                XmlEvent::EndElement { name } => {
                    if name.local_name == "GamePaths" {
                        in_game_paths = false;
                    } else if name.local_name == "Entry" && in_entry {
                        if !current_entry.is_empty() {
                            game_paths.push(current_entry.clone());
                            current_entry.clear();
                        }
                        in_entry = false;
                    }
                }
                XmlEvent::Characters(text) => {
                    if in_entry {
                        current_entry.push_str(&text);
                    }
                }
                _ => {}
            }
        }

        Ok(CemuConfig { game_paths })
    }

    fn create_default_config(&self, cemu_cfg: &CemuConfig) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = self.ukmm_config.parent() {
            fs::create_dir_all(parent)?;
        }

        let graphics_packs_path = self.home.join(".local/share/Cemu/graphicPacks");
        let storage_dir = self.home.join(".local/share/ukmm");

        // Use Cemu's game path as host_path if available
        let host_path = cemu_cfg
            .game_paths
            .first()
            .map(|s| s.as_str())
            .unwrap_or("");

        let host_path_line = if !host_path.is_empty() {
            format!("host_path: {}", host_path)
        } else {
            "host_path:".to_string()
        };

        let yaml_content = format!(
            r#"current_mode: WiiU
system_7z: true
storage_dir: {}
check_updates: Stable
show_changelog: true
wiiu_config:
  language: USen
  profile: Default
  dump:
    bin_type: Nintendo
    source:
      type: Unpacked
      {}
      content_dir:
      update_dir:
      aoc_dir:
    endian: Wii U
  deploy_config:
    output: {}
    layout: WithName
    method: Copy
switch_config:
lang: English
"#,
            storage_dir.display(),
            host_path_line,
            graphics_packs_path.display()
        );

        fs::write(&self.ukmm_config, yaml_content)?;
        println!(
            "✓ Created default UKMM config at: {}",
            self.ukmm_config.display()
        );
        if !host_path.is_empty() {
            println!("  host_path: {}", host_path);
        }
        println!("  Note: Game dump paths (content_dir, update_dir, aoc_dir) are still null");
        println!("  Please configure them in UKMM's GUI or manually edit the config file");
        Ok(())
    }

    fn load_ukmm_config(&self, cemu_cfg: &CemuConfig) -> Result<UkmConfig, Box<dyn std::error::Error>> {
        if !self.ukmm_config.exists() {
            println!("⚠ UKMM config not found, creating default configuration...\n");
            self.create_default_config(cemu_cfg)?;
            println!();
        }

        let file = fs::File::open(&self.ukmm_config)?;
        let config: Value = serde_yaml::from_reader(file)?;

        let wiiu_config = config
            .get("wiiu_config")
            .and_then(|v| v.as_mapping());

        let dump = wiiu_config
            .and_then(|m| m.get("dump"))
            .and_then(|v| v.as_mapping());

        let source = dump
            .and_then(|m| m.get("source"))
            .and_then(|v| v.as_mapping());

        let host_path = source
            .and_then(|m| m.get("host_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let content_dir = source
            .and_then(|m| m.get("content_dir"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let update_dir = source
            .and_then(|m| m.get("update_dir"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let aoc_dir = source
            .and_then(|m| m.get("aoc_dir"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let deploy_config = wiiu_config.and_then(|m| m.get("deploy_config")).and_then(|v| {
            v.as_mapping().map(|m| {
                DeployConfig {
                    output: m.get("output")
                        .and_then(|val| val.as_str())
                        .map(|s| s.to_string()),
                    layout: m.get("layout")
                        .and_then(|val| val.as_str())
                        .map(|s| s.to_string()),
                    method: m.get("method")
                        .and_then(|val| val.as_str())
                        .map(|s| s.to_string()),
                }
            })
        });

        Ok(UkmConfig {
            host_path,
            content_dir,
            update_dir,
            aoc_dir,
            deploy_config,
        })
    }

    fn path_exists(&mut self, path: &str, name: &str) -> bool {
        let p = Path::new(path);
        if p.is_dir() {
            println!("  ✓ {}", name);
            true
        } else {
            println!("  ❌ {} NOT FOUND: {}", name, path);
            self.errors += 1;
            false
        }
    }

    fn file_exists(&mut self, path: &Path, name: &str) -> bool {
        if path.is_file() {
            println!("    ✓ {}", name);
            true
        } else {
            println!("    ❌ {} NOT FOUND: {}", name, path.display());
            self.errors += 1;
            false
        }
    }

    fn run(mut self) -> i32 {
        println!("=== Configuration Verification ===\n");

        let cemu_cfg = match self.load_cemu_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Error loading Cemu config: {}", e);
                return 1;
            }
        };

        let ukmm_cfg = match self.load_ukmm_config(&cemu_cfg) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Error loading UKMM config: {}", e);
                return 1;
            }
        };

        println!("✓ Both config files found\n");

        // Display Cemu configuration
        println!("=== Cemu Configuration ===");
        for (i, path) in cemu_cfg.game_paths.iter().enumerate() {
            println!("Game Path {}: {}", i + 1, path);
        }
        println!();

        // Display UKMM dump configuration
        println!("=== UKMM Dump Configuration ===");
        println!("Host Path:    {:?}", ukmm_cfg.host_path);
        println!("Content Dir:  {:?}", ukmm_cfg.content_dir);
        println!("Update Dir:   {:?}", ukmm_cfg.update_dir);
        println!("AOC Dir:      {:?}", ukmm_cfg.aoc_dir);
        println!();

        // Display UKMM deployment configuration
        println!("=== UKMM Deployment Configuration ===");
        if let Some(ref deploy) = ukmm_cfg.deploy_config {
            println!(
                "Deploy Path:   {}",
                deploy.output.as_ref().unwrap_or(&"N/A".to_string())
            );
            println!(
                "Deploy Layout: {}",
                deploy.layout.as_ref().unwrap_or(&"N/A".to_string())
            );
            println!(
                "Deploy Method: {}",
                deploy.method.as_ref().unwrap_or(&"N/A".to_string())
            );
        } else {
            println!("⚠ No deployment configuration set");
        }
        println!();

        // Verify paths exist
        println!("=== Path Verification ===");
        let cemu_path_valid = if let Some(path) = cemu_cfg.game_paths.first() {
            self.path_exists(path, "Cemu game path")
        } else {
            false
        };

        if let Some(path) = &ukmm_cfg.host_path {
            self.path_exists(path, "UKMM host path");
        }
        if let Some(path) = &ukmm_cfg.content_dir {
            self.path_exists(path, "UKMM content dir");
        }
        if let Some(path) = &ukmm_cfg.update_dir {
            self.path_exists(path, "UKMM update dir");
        }
        if let Some(path) = &ukmm_cfg.aoc_dir {
            self.path_exists(path, "UKMM DLC dir");
        }
        println!();

        // Verify game dump structure
        println!("=== Game Dump Verification ===");
        if let Some(ref content_path) = ukmm_cfg.content_dir {
            let content_dir = Path::new(content_path);
            if content_dir.is_dir() {
                self.file_exists(
                    &content_dir.join("Pack/Dungeon001.pack"),
                    "Base game Dungeon001.pack",
                );
            }
        }

        if let Some(ref aoc_path) = ukmm_cfg.aoc_dir {
            let aoc_dir = Path::new(aoc_path);
            if aoc_dir.is_dir() {
                self.file_exists(
                    &aoc_dir.join("Pack/AocMainField.pack"),
                    "DLC AocMainField.pack",
                );
            }
        }
        println!();

        // Check deployment configuration
        println!("=== Deployment Verification ===");
        let expected_deploy_path = self.home.join(".local/share/Cemu/graphicPacks");

        if let Some(deploy_config) = &ukmm_cfg.deploy_config {
            if let Some(ref output) = deploy_config.output {
                let deploy_path = Path::new(output);

                if deploy_path == expected_deploy_path {
                    println!("✓ Deployment path points to Cemu graphics packs");
                } else {
                    println!("⚠ Deployment path is not standard Cemu location:");
                    println!("  Expected: {}", expected_deploy_path.display());
                    println!("  Actual:   {}", deploy_path.display());
                }

                if let Some(ref layout) = deploy_config.layout {
                    if layout == "WithName" {
                        println!("✓ Deployment layout is 'WithName' (correct for Cemu)");
                    } else {
                        println!(
                            "❌ Deployment layout is '{}' (should be 'WithName')",
                            layout
                        );
                        self.errors += 1;
                    }
                }

                if deploy_path.exists() {
                    println!("✓ Deployment path exists");
                } else {
                    println!("❌ Deployment path does not exist: {}", deploy_path.display());
                    self.errors += 1;
                }
            }
        } else {
            println!("❌ No deployment configuration set");
            self.errors += 1;
        }
        println!();

        // Check alignment between Cemu and UKMM
        println!("=== Configuration Alignment ===");
        if cemu_path_valid {
            if let Some(cemu_path) = cemu_cfg.game_paths.first() {
                if let Some(ref ukmm_path) = ukmm_cfg.host_path {
                    if cemu_path == ukmm_path {
                        println!("✓ Cemu and UKMM point to the same root directory");
                    } else {
                        println!("⚠ Cemu and UKMM have different root paths:");
                        println!("  Cemu: {}", cemu_path);
                        println!("  UKMM: {}", ukmm_path);
                    }
                }
            }
        }
        println!();

        // Final status
        if self.errors == 0 {
            println!("=== All checks passed! ===");
            0
        } else {
            println!("=== {} error(s) found ===", self.errors);
            1
        }
    }
}
