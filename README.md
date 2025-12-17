# 🐻 Memoro Vault 'Open Cracker'

**By @SlowBearDigger**

Hey guys, this is a tool I put together to help crack the Memoro Vault challenge. It's built in Rust so it's blazing fast, and I've set it up so you don't need to be a coder to use it. You just edit a config file and run it.

## 🚀 Step 1: Install Rust (If you don't have it)

If you're new to this, you need the Rust compiler. It's super easy.
1.  Go to [rustup.rs](https://rustup.rs/).
2.  Copy the command they give you and paste it into your Terminal (Mac/Linux) or download the installer (Windows).
3.  Restart your terminal.

## 📁 Step 2: Setup

1. Unzip this folder.
2. **IMPORTANT:** You need the `vault.meta` file from the challenge. Finding it is part of the fun (Hint: It's inside the Electron app resources).
3. Copy `vault.meta` into this folder (next to `cracker.conf`).

## ⚙️ Step 3: Configure Your Attack

Open `cracker.conf` with any text editor (Notepad, VS Code, etc).

1.  **[PROFILE] Section:** These are the 25 answers we *think* are correct. I've pre-filled it with the best public info we have (Auburn profile, etc.).
2.  **[ATTACKS] Section:** This is where the magic happens.
    *   If you think we have the wrong Maiden Name (Question #3, Index 2), point it to a dictionary file:
        `2=dictionaries/surnames.txt`
    *   The script will take your Base Profile, swap out Answer #3 with every word in that text file, and test it.

## 🔥 Step 4: Run It

Open your terminal in this folder and type:

```bash
cargo run --release
```

That's it. It will compile (takes a minute the first time) and then start churning through combinations.

## 📝 Tips
- The tool handles all the crazy internal sorting/hashing for you.
- It uses the exact crypto settings from the Vault (Argon2id + AES-GCM).
- Edit the text files in `dictionaries/` to add your own guesses!

Happy digging! 🐻
