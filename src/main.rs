
use aes_gcm::{Aes256Gcm, KeyInit, Key, Nonce};
use aes_gcm::aead::Aead;
use argon2::{Argon2, Algorithm, Version, Params};
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::sync::Arc;
use std::io::{self, BufRead, Write};
use std::fs::File;

// --- CONFIGURATION ---
// I've set this up so it reads from a simple text file. 
// You don't need to touch the code to change the attack target.
const VAULT_META: &str = "vault.meta"; // Make sure your vault.meta is in the same folder!
const CONFIG_FILE: &str = "cracker.conf";

// This is the Salt & IV extracted from the main vault.json.
// If you're attacking a different vault, you might need to change these, 
// but for the Memoro Bounty, these are constant.
const FULL_SALT_HEX: &str = "71bbae156fbc70478248fd07404be630";
const META_IV: [u8; 12] = [41, 37, 64, 72, 163, 14, 253, 248, 98, 102, 149, 218];

// The Vault asks questions in a specific scrambled order. 
// I mapped this out so we construct the password correctly every time.
const ORDER: [usize; 25] = [22, 23, 21, 0, 19, 3, 24, 4, 16, 9, 13, 7, 20, 12, 11, 15, 14, 10, 2, 6, 1, 17, 5, 18, 8];

fn main() {
    println!("---------------------------------------------------");
    println!("🐻 Memoro Vault 'Open Cracker' by @SlowBearDigger 🐻");
    println!("---------------------------------------------------");
    println!(">> Digging for configuration in '{}'...", CONFIG_FILE);

    // Load up the plan...
    let config = load_config(CONFIG_FILE).expect("Hey, I couldn't find 'cracker.conf'. Did you delete it?");
    let base_profile = config.base_profile;
    let attacks = config.attacks;

    // Sanity check: We need exactly 25 base answers to work with.
    if base_profile.len() != 25 {
        panic!("Whoops! The Base Profile in cracker.conf must have exactly 25 lines. I found {}.", base_profile.len());
    }

    println!(">> Layout loaded. Ready to dig.");
    
    let mut combos: Vec<[String; 25]> = Vec::new();

    // Here's the magic. We take your dictionaries and inject them into the specific questions you want to attack.
    for attack in attacks {
        println!("   -> Target identified: Question #{} (Index {}). Reading dictionary '{}'...", attack.index + 1, attack.index, attack.dict_path);
        
        match load_lines(&attack.dict_path) {
            Ok(words) => {
                println!("      Got {} possibilities.", words.len());
                for word in words {
                    // Start with the base profile...
                    let mut candidate = base_profile.clone();
                    // ...and swap in the dictionary word.
                    candidate[attack.index] = word;
                    combos.push(candidate);
                }
            },
            Err(_) => println!("      [!] Warning: Couldn't read '{}'. Skipping this list.", attack.dict_path),
        }
    }

    if combos.is_empty() {
        println!("\n[!] No combinations generated. Check your config file. I need something to work with!");
        return;
    }

    println!("\n>> Loaded {} combinations. Let's get to work.", combos.len());
    
    // --- THE ENGINE ---
    // Using Argon2id + AES-GCM, exactly how the Vault does it.
    let full_salt = hex::decode(FULL_SALT_HEX).expect("Hex Salt is broken. Check code.");
    
    // We try to read vault.meta from the current folder.
    let meta_content = match std::fs::read(VAULT_META) {
        Ok(c) => c,
        Err(_) => {
            println!("\n[!] CRITICAL: I can't find 'vault.meta'.");
            println!("    Please copy the 'vault.meta' file from the challenge into this folder.");
            return;
        }
    };

    // Performance settings (Hardcoded to match Memoro Vault v1.0.8)
    let time_cost = 3;
    let mem_kib = 16384;
    let parallelism = 1;
    let hash_len = 32;

    let attempts = AtomicUsize::new(0);
    let found = Arc::new(AtomicBool::new(false));
    let start = std::time::Instant::now();

    println!(">> Engine Start. Press Ctrl+C if you need to stop.\n");

    // Multi-threading go brrr
    combos.par_iter().for_each(|answers| {
        if found.load(Ordering::Relaxed) { return; }

        let c = attempts.fetch_add(1, Ordering::Relaxed);
        
        // Status update every 500 attempts so you know it's alive
        if c % 500 == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            let rate = c as f64 / elapsed;
            print!("\r>> Digging... {}/{} ({:.0} kh/s) | {:.1}%", c, combos.len(), rate, (c as f64 / combos.len() as f64) * 100.0);
            io::stdout().flush().ok();
        }

        // 1. Arrange answers in the Crypto Order
        let password: String = ORDER.iter()
            .map(|&i| answers[i].trim().to_lowercase())
            .collect::<Vec<_>>()
            .join("");

        // 2. Derive the Encryption Key (Argon2id)
        if let Ok(key) = derive_key(password.as_bytes(), &full_salt, time_cost, mem_kib, parallelism, hash_len) {
            // 3. Try to unlock the door (AES-GCM)
            if let Ok(decrypted) = decrypt(&key, &META_IV, &meta_content) {
                // 4. Did it open?
                let text = String::from_utf8_lossy(&decrypted);
                // "files" is the JSON key inside the decrypted metadata. If we see it, we're in.
                if text.contains("\"files\"") {
                    found.store(true, Ordering::Relaxed);
                    println!("\n\n🎉 BOOM! WE'RE IN! 🎉");
                    println!(">> The Password is: {}", password);
                    println!("---------------------------------------------------");
                    println!(">> Decrypted Data Preview:\n{}", text);
                    println!("---------------------------------------------------");
                    std::process::exit(0);
                }
            }
        }
    });

    if !found.load(Ordering::Relaxed) {
        println!("\n\n>> Job done. No match found in this run.");
        println!(">> Try adding more words to your dictionaries or checking the Base Profile.");
    }
}

// --- CONFIG STUFF ---
// Simple parser so we don't need heavy dependencies.

struct Config {
    base_profile: [String; 25],
    attacks: Vec<Attack>,
}

struct Attack {
    index: usize,
    dict_path: String,
}

fn load_config(path: &str) -> io::Result<Config> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);
    
    let mut base_profile = Vec::new();
    let mut attacks = Vec::new();
    let mut reading_profile = false;

    for line in reader.lines() {
        let line = line?;
        let trim = line.trim();
        // Skip comments and empty lines
        if trim.is_empty() || trim.starts_with('#') { continue; }

        if trim == "[PROFILE]" {
            reading_profile = true;
            continue;
        } else if trim == "[ATTACKS]" {
            reading_profile = false;
            continue;
        }

        if reading_profile {
            base_profile.push(trim.to_string());
        } else {
            // Format: INDEX=path/to/dict.txt
            if let Some((idx_str, path)) = trim.split_once('=') {
                if let Ok(index) = idx_str.trim().parse::<usize>() {
                    attacks.push(Attack { 
                        index: index, 
                        dict_path: path.trim().to_string() 
                    });
                }
            }
        }
    }

    if base_profile.len() != 25 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Profile invalid"));
    }

    let arr: [String; 25] = base_profile.try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Profile conversion failed"))?;

    Ok(Config {
        base_profile: arr,
        attacks,
    })
}


fn load_lines(path: &str) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    Ok(io::BufReader::new(file)
        .lines()
        .filter_map(Result::ok)
        .map(|l| l.trim().to_lowercase())
        .filter(|l| !l.is_empty())
        .collect())
}

// --- CRYPTO LOGIC ---
// Standard implementation of what's used in the Vault.

fn derive_key(password: &[u8], salt: &[u8], time: u32, mem: u32, par: u32, len: u32) -> Result<Vec<u8>, String> {
    let params = Params::new(mem, time, par, Some(len as usize)).map_err(|e| format!("{:?}", e))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = vec![0u8; len as usize];
    
    // Note: The vault uses Base64 encoded salt for some reason. We replicate that here.
    use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
    let salt_b64 = STANDARD_NO_PAD.encode(salt);
    
    argon2.hash_password_into(password, salt_b64.as_bytes(), &mut out).map_err(|e| format!("{:?}", e))?;
    Ok(out)
}

fn decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce);
    cipher.decrypt(nonce, ciphertext).map_err(|_| "Decrypt failed".into())
}
