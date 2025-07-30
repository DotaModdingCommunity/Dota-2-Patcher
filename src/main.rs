use std::fs::{create_dir, exists, write, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;
use std::env;
use std::process::{exit, Command};
use sha1::{Sha1, Digest};
use crc::Crc;
use steamlocate::SteamDir;


fn get_paths() -> (PathBuf, PathBuf, PathBuf) {
    let steam_dir = SteamDir::locate().expect("Could not find Steam installation");
    
    let (dota2, lib) = steam_dir.find_app(570).expect("Could not find Dota 2").expect("Dota 2 is not installed");

    let game_path = lib.path().join("steamapps").join("common").join(&dota2.install_dir);
    let gameinfo_path = PathBuf::new().join(&game_path).join("game").join("dota").join("gameinfo_branchspecific.gi");
    let dota_signatures_path = PathBuf::new().join(&game_path).join("game").join("bin").join("win64").join("dota.signatures");
    let mod_dir_path = PathBuf::new().join(&game_path).join("game").join("DotaModdingCommunityMods");

    return (gameinfo_path, dota_signatures_path, mod_dir_path)
}

fn is_dota2_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("tasklist")
            .output()
            .expect("Failed to execute tasklist");
        let tasks = String::from_utf8_lossy(&output.stdout);
        tasks.contains("dota2.exe")
    }

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("pgrep")
            .arg("dota2")
            .output()
            .expect("Failed to execute pgrep");
        !output.stdout.is_empty()
    }

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("pgrep")
            .arg("Dota 2")
            .output()
            .expect("Failed to execute pgrep");
        !output.stdout.is_empty()
    }
}

fn validate_patch_state(gameinfo_path:&PathBuf, dota_signatures_path:&PathBuf) -> (bool, bool){
    let mut gameinfo_patched=false;
    let mut dota_signatures_patched=false;

    let mut file = File::open(gameinfo_path).expect("Unable to open gameinfo_branchspecific.gi");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Unable to read gameinfo_branchspecific.gi");
    if contents.find("// Patched by DotaModdingCommunity Patcher") == None {
    } else {
        gameinfo_patched = true;
    }

    let mut file = File::open(dota_signatures_path).expect("Unable to open dota.signatures");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Unable to read dota.signatures");
    let last_line = contents.lines().last().expect("Unable to find last line");
    if last_line.starts_with("...") {
        let (actual_sha1, actual_crc32) = calculate_hashes(&gameinfo_path);
        let parts: Vec<&str> = last_line.split('~').collect();
        let info = parts[1];
        let info_parts: Vec<&str> = info.split(';').collect();
        let sha1_part = info_parts[0];
        let crc_part = info_parts[1];
        let sha1 = sha1_part.split(':').nth(1).unwrap().trim().to_string();
        let crc32 = crc_part.split(':').nth(1).unwrap().trim().to_string();
        if actual_crc32 == crc32 && actual_sha1 == sha1 {
            dota_signatures_patched = true;
        }
    }
    
    return (gameinfo_patched, dota_signatures_patched);
}

fn backup_gameinfo(gameinfo_path: &PathBuf){
    let mut backup = gameinfo_path.clone();
    backup.set_extension("gi_backup");
    if !exists(&backup).expect("Unable to verify if file exists") {
        std::fs::copy( gameinfo_path, backup).expect("Unable to make backup");
    }
}

fn backup_dota_signatures(dota_signatures_path: &PathBuf){
    let mut backup = dota_signatures_path.clone();
    backup.set_extension("signatures_backup");
    if !exists(&backup).expect("Error") {
        std::fs::copy( dota_signatures_path, backup).expect("Unable to make backup");
    }
}

fn modify_gameinfo(gameinfo_path: &PathBuf) {
    let mut file = File::open(gameinfo_path).expect("Unable to open gameinfo_branchspecific.gi");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Unable to read gameinfo_branchspecific.gi");
    let mut lines: Vec<&str> = contents.lines().collect();
    let line_index = lines.len() - 2;
    let insert = r#"		SearchPaths // Patched by DotaModdingCommunity Patcher
		{
			Game_Language		dota_*LANGUAGE*

			Game_LowViolence	dota_lv

			Game				DotaModdingCommunityMods
			Game				dota
			Game				core

			Mod					DotaModdingCommunityMods
			Mod					dota

			Write				dota

			AddonRoot_Language	dota_*LANGUAGE*_addons

			AddonRoot			dota_addons

			PublicContent		dota_core
			PublicContent		core
		}"#;
    lines.insert(line_index, insert);
    let new_content = lines.join("\n");
    write(gameinfo_path, new_content).expect("Unable to write changes to gameinfo_branchspecifi.gi");
}

fn calculate_hashes(gameinfo_path: &PathBuf) -> (String, String) {
    let mut file = File::open(gameinfo_path).expect("Unable to open gameinfo_branchspecific.gi");
    let mut sha1_hasher = Sha1::new();
    let crc = Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
    let mut crc_digest = crc.digest();

    let mut buffer = [0; 4096];
    loop {
        let n = file.read(&mut buffer).expect("Failed to read file");
        if n == 0 {
            break;
        }
        sha1_hasher.update(&buffer[..n]);
        crc_digest.update(&buffer[..n]);
    }

    let checksum = crc_digest.finalize();

    let sha1_result = sha1_hasher.finalize();
    let sha1_hex = sha1_result.iter().map(|b| format!("{:02X}", b)).collect::<String>();

    let crc_bytes = checksum.to_le_bytes();
    let crc_hex = crc_bytes.iter().map(|b| format!("{:02X}", b)).collect::<String>();

    return (sha1_hex, crc_hex);
}

fn modify_dota_signatures(dota_signatures_path: &PathBuf, sha1:String, crc32:String) {
    let mut file = File::open(dota_signatures_path).expect("Unable to open dota.signatures");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Unable to read dota.signatures");
    let patch = (r#"...\..\..\dota\gameinfo_branchspecific.gi~SHA1:"#).to_string() + &sha1 + ";CRC:" + &crc32;
    contents.push_str("\n");
    contents.push_str(&patch);
    write(dota_signatures_path, contents).expect("Unable to write changes to dota.signatures")
}
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() <2 {
        println!(r#"   Dota Modding Community Patcher v 1.0.0
▒▒▒▒▒▒▒▒▒▒▒     ▒▒▒▒▒▒▒▒▒▒▒   ░▒▒▒▒▒▒▒▒▒▒▒▒▒ 
░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
░░▒▒▒▒▒▒▒▒▒▒▒░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░▒▒▒▒▒▒▒▒▒▒▒▒
░░▒▒▒▒▒▒▒▒▒▒░░░▒░▒▒▒▒▒▒▒▒▒▒▒▒░▒░░▒▒▒▒▒▒▒▒▒▒▒▒
░░▒▒▒▒▒▒▒▒▒▒░░░▒░░▒▒▒▒▒▒▒▒▒▒░░▒░░░▒▒▒▒▒▒▒▒▒▒▒
░░▒▒▒▒▒▒▒▒▒░░░░▒░░▒▒▒▒▒▒▒▒▒░░░▒░░░▒▒▒▒▒▒▒▒▒▒▒
░░▒▒▒▒▒▒▒▒▒░░░░░░░░▒▒▒▒▒▒▒▒░░░▒░░░░▒▒▒▒▒▒▒▒▒▒
░░▒▒▒▒▒▒▒▒░░░░░░░░░▒▒▒▒▒▒▒░░░░▒░░░░▒▒▒▒▒▒▒▒░▒
░░░▒▒▒▒▒▒▒░░░░▒▒░░░░▒▒▒▒▒░░░░░▒░░░░░▒▒▒▒▒▒▒▒ 
░░░▒▒▒▒▒▒░░░░░▒▒░░░░░▒▒▒▒░░░░▒▒░░░░░▒▒▒▒▒▒▒▓ 
░░░▒▒▒▒▒▒░░░░░▒▒▒░░░░▒▒▒░░░░░▒▒░░░░░░▒▒▒▒▒▒▒▒
 ░░▒▒▒▒▒░░░░░░▒▒▒▒░░░░▒░░░░░▒▒▒▒░░░░░▒▒▒▒▒▒▒▒
 ░░░▒▒▒▒░░░░░░▒▒▒▒▒░░░▒░░░░▒▒▒▒▒░░░░░░▒▒▒▒▒▒▒
░▒▒▒▒▒▒▒░░░░░░▒▒▒▒▒░░░▒░░░▒▒▒▒▒▒░░░░░░▒▒▒▒▒▒ 
░░░▒▒▒▒░░░░░░░▒▒▒▒▒▒░░▒░░░▒▒▒▒▒▒░░░░░░░▒▒▒▒▒▒
 ░░▒▒▒▒░░░░░░░▒▒▒▒▒▒▒░▒░░▒▒▒▒▒▒▒░░░░░░░▒▒▒▒▒▒
 ░░░▒▒░░░░░░░▒▒▒▒▒▒▒▒░▒░▒▒▒▒▒▒▒▒░░░░░░░▒▒▒▒▒▒
 ░░▒▒▒░░░░░░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░▒▒▒▒▒
 ░░▒▒░░░░░░░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░▒▒▒▒▒
░░░▒▒▒▒░░░░░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░▒▒▒▒▒
░░░▒▒▒▒▒▒░░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░▒▒▒▒▒▒▒▒
░░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
░░░░░░░░░░░░░░░░░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░▒
  ░░            ░░░░░░░░░░░░░░░░░░░░░░░░░    "#);
        if is_dota2_running() {
            println!("Dota 2 is currently running! Close it and try again.");
            sleep(Duration::from_secs(8));
            exit(1)
        }
        let (gameinfo_path, dota_signatures_path, mod_dir_path) = get_paths();
        let (gameinfo_patched,dota_signatures_patched) = validate_patch_state(&gameinfo_path, &dota_signatures_path);
        if !gameinfo_patched {
            backup_gameinfo(&gameinfo_path);
            modify_gameinfo(&gameinfo_path);
        }
        if !dota_signatures_patched {
            backup_dota_signatures(&dota_signatures_path);
            let (sha1, crc32) = calculate_hashes(&gameinfo_path);
            modify_dota_signatures(&dota_signatures_path, sha1, crc32);
        }
        let mod_dir_exists = Path::new(&mod_dir_path).try_exists().expect("Unable to veryfy mod directory");
        if !mod_dir_exists {
            create_dir(Path::new(&mod_dir_path)).expect("Unable to create mod directory");
        }
        
        println!(r#"
Patch successfull!
Window will close automaticaly...
"#);
        sleep(Duration::from_secs(8));
    } else {
        let (gameinfo_path, dota_signatures_path, _mod_dir_path) = get_paths();
        let (gameinfo_patched,dota_signatures_patched) = validate_patch_state(&gameinfo_path, &dota_signatures_path);
        if !gameinfo_patched {
            backup_gameinfo(&gameinfo_path);
            modify_gameinfo(&gameinfo_path);
        }
        if !dota_signatures_patched {
            backup_dota_signatures(&dota_signatures_path);
            let (sha1, crc32) = calculate_hashes(&gameinfo_path);
            modify_dota_signatures(&dota_signatures_path, sha1, crc32);
        }
        let game_executable = &args[1];
        let game_args = &args[2..];
        let mut cmd = Command::new(game_executable);
        cmd.args(game_args);
        match cmd.spawn() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to start game: {}", e);
            }
        }
    }
}