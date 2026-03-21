# Iris Security Threat Model

Security considerations and threat mitigation for Iris terminal emulator.

## Threat Categories

| Category | Risk Level | Priority |
|----------|------------|----------|
| Input Handling | Critical | P0 |
| PTY Security | High | P0 |
| SSH Credentials | High | P0 |
| Clipboard | Medium | P1 |
| Network (SSH) | High | P0 |
| File Access | Medium | P1 |
| Dependencies | Medium | P1 |
| Persistence | Low | P2 |

---

## Input Handling

### Threat: Malicious Escape Sequences

Terminal escape sequences can manipulate the terminal state in unexpected ways.

**Attack Vectors:**
- `OSC 4` - Set color palette (phishing via fake prompts)
- `OSC 7` - Set working directory (path traversal)
- `OSC 9` - Windows notification spam
- `OSC 52` - Clipboard write (data exfiltration)
- `OSC 777` - Desktop notification spam
- `DCS` queries - Terminal identity probing
- `DECSET` abuse - Unexpected mode changes

**Example Attack:**
```bash
# Malicious script writes to clipboard via OSC 52
echo -e '\e]52;c;$(cat ~/.ssh/id_rsa | base64)\e\\'
```

**Mitigations:**

1. **Escape Sequence Filtering**:
   ```rust
   pub struct EscapeFilter {
       allowed_osc: HashSet<u16>,
       clipboard_write_requires_prompt: bool,
       notification_requires_prompt: bool,
   }
   
   impl EscapeFilter {
       pub fn filter(&self, seq: &EscapeSequence) -> Option<EscapeSequence> {
           match seq {
               EscapeSequence::Osc(code, _) => {
                   if self.allowed_osc.contains(code) {
                       Some(seq.clone())
                   } else {
                       tracing::warn!(code, "Blocked OSC sequence");
                       None
                   }
               }
               _ => Some(seq.clone()),
           }
       }
   }
   ```

2. **Default Allowed Sequences**:
   ```rust
   // Safe by default
   let allowed = [
       0,    // Set window title
       4,    // Set color (with prompt)
       10,   // Set foreground color
       11,   // Set background color
       104,  // Reset color
       110,  // Reset foreground
       111,  // Reset background
   ];
   ```

3. **User Confirmation for Sensitive Operations**:
   ```rust
   // OSC 52 clipboard write
   pub fn handle_clipboard_write(&mut self, data: &[u8]) -> Result<(), Error> {
       if self.config.clipboard_prompt {
           let decoded = base64::decode(data)?;
           if self.prompt_user("Allow clipboard write?", &decoded)? {
               self.clipboard.set(&decoded)?;
           }
       } else {
           self.log_clipboard_write();
           self.clipboard.set(&base64::decode(data)?)?;
       }
       Ok(())
   }
   ```

4. **Resource Limits**:
   ```rust
   pub const MAX_OSC_ARG_SIZE: usize = 4096;      // 4KB per argument
   pub const MAX_SEQUENCE_SIZE: usize = 65536;   // 64KB total sequence
   pub const MAX_PARAMS: usize = 16;             // Max CSI parameters
   pub const MAX_TITLE_LENGTH: usize = 256;       // Window title
   ```

### Threat: Binary Data Injection

Binary data passed through the terminal can corrupt state or crash the parser.

**Attack Vectors:**
- NUL bytes truncating strings
- Invalid UTF-8 sequences
- Control characters in unexpected locations
- Extremely long lines (memory exhaustion)

**Mitigations:**

```rust
pub struct InputSanitizer {
    max_line_length: usize,
    max_consecutive_control: usize,
}

impl InputSanitizer {
    pub fn sanitize(&self, input: &[u8]) -> Cow<[u8]> {
        let mut output = Vec::with_capacity(input.len());
        let mut control_count = 0;
        let mut line_length = 0;
        
        for &byte in input {
            // Limit line length
            if line_length >= self.max_line_length {
                continue;
            }
            
            // Handle control characters
            if byte < 0x20 || byte == 0x7F {
                control_count += 1;
                if control_count <= self.max_consecutive_control {
                    output.push(byte);
                }
                if byte == b'\n' {
                    line_length = 0;
                }
            } else {
                control_count = 0;
                line_length += 1;
                output.push(byte);
            }
        }
        
        output.into()
    }
}
```

### Threat: Unicode Normalization Attacks

Unicode has multiple ways to represent the same character, enabling bypass attacks.

**Attack Vectors:**
- Homoglyphs ( Cyrillic 'а' vs Latin 'a')
- Zero-width characters hiding malicious content
- Right-to-left override for visual spoofing
- Combining characters for visual spoofing

**Example Attack:**
```bash
# Fake command using homoglyphs
# Cyrillic 'а' (U+0430) instead of Latin 'a' (U+0061)
echo "rm -rf /"  # Looks like: rm -rf /
```

**Mitigations:**

```rust
pub fn normalize_unicode(input: &str) -> String {
    // NFKC normalization catches most homoglyphs
    unicode_normalization::nfkc(input).collect()
}

pub fn audit_unicode(text: &str) -> Vec<UnicodeWarning> {
    let mut warnings = Vec::new();
    for (i, c) in text.char_indices() {
        // Warn on zero-width characters
        if is_zero_width(c) {
            warnings.push(UnicodeWarning::ZeroWidth { pos: i });
        }
        // Warn on RTL override
        if c == '\u{202E}' {
            warnings.push(UnicodeWarning::RtlOverride { pos: i });
        }
        // Warn on combining characters at line start
        if i == 0 && unicode_normalization::is_combining(c) {
            warnings.push(UnicodeWarning::LeadingCombining { pos: i });
        }
    }
    warnings
}
```

### Threat: Buffer Overflow via Long Sequences

Long escape sequences can exhaust memory or overflow buffers.

**Attack Vectors:**
- 1MB+ OSC arguments
- Deeply nested DCS sequences
- Infinite loop character attributes
- Scrollback abuse (billions of lines)

**Mitigations:**

```rust
pub struct ParserLimits {
    pub max_sequence_bytes: usize,      // 64KB max sequence
    pub max_osc_args: usize,            // 16 arguments max
    pub max_nesting_depth: usize,       // 8 levels max
    pub max_scrollback_lines: usize,    // Configurable limit
}

impl Default for ParserLimits {
    fn default() -> Self {
        Self {
            max_sequence_bytes: 65536,
            max_osc_args: 16,
            max_nesting_depth: 8,
            max_scrollback_lines: 100_000,
        }
    }
}

impl Parser {
    pub fn parse(&mut self, input: &[u8]) -> Result<Vec<Action>, Error> {
        // Enforce limits
        if input.len() > self.limits.max_sequence_bytes {
            return Err(Error::SequenceTooLarge {
                size: input.len(),
                max: self.limits.max_sequence_bytes,
            });
        }
        // ...
    }
}
```

---

## PTY Security

### Threat: Process Escape via Terminal Commands

Some terminal features can launch processes or modify the environment.

**Attack Vectors:**
- `OSC 7` - Set working directory (potential path traversal)
- `OSC 777` - Launch desktop notifications
- Shell escape via `$()` in window title
- Environment variable manipulation

**Mitigations:**

```rust
pub fn validate_osc_7(&self, path: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(path);
    
    // Normalize path
    let canonical = path.canonicalize()
        .map_err(|_| Error::InvalidPath { path: path.to_string() })?;
    
    // Restrict to allowed directories
    for allowed in &self.config.allowed_directories {
        if canonical.starts_with(allowed) {
            return Ok(canonical);
        }
    }
    
    Err(Error::PathNotAllowed { path: path.to_string() })
}
```

### Threat: Privilege Escalation

Processes running in the terminal may gain elevated privileges.

**Attack Vectors:**
- `sudo` password capture via fake prompts
- Running setuid binaries in terminal
- Credential harvesting via keylogging patterns
- Session hijacking

**Mitigations:**

```rust
pub struct PrivilegeMonitor {
    sudo_active: bool,
    last_prompt_hash: Option<u64>,
}

impl PrivilegeMonitor {
    pub fn check_prompt(&mut self, line: &str) {
        // Detect sudo password prompts
        if line.contains("[sudo]") || line.ends_with("Password:") {
            self.sudo_active = true;
            tracing::warn!("Privilege escalation detected");
        }
        
        // Clear sensitive data from scrollback after use
        if self.sudo_active && line.contains("authentication failure") {
            // Don't log password attempts
        }
    }
}
```

### Threat: Shell Injection via Terminal

Terminal features can inject commands into the shell.

**Attack Vectors:**
- Bracketed paste manipulation
- OSC 133 shell integration abuse
- Modified environment variables
- Alias injection

**Mitigations:**

```rust
pub fn validate_shell_integration(&self, seq: &ShellSequence) -> Result<(), Error> {
    match seq {
        ShellSequence::SetVar(name, value) => {
            // Block dangerous variables
            let blocked = ["PATH", "LD_PRELOAD", "LD_LIBRARY_PATH", "SHELL"];
            if blocked.contains(&name.as_str()) {
                return Err(Error::BlockedVariable { name: name.clone() });
            }
            Ok(())
        }
        ShellSequence::Command(cmd) => {
            // Only allow safe commands
            let allowed = ["cd", "echo", "export"];
            let command = cmd.split_whitespace().next().unwrap_or("");
            if !allowed.contains(&command) {
                return Err(Error::BlockedCommand { command: command.to_string() });
            }
            Ok(())
        }
    }
}
```

---

## SSH Credentials

### Threat: Password Storage

SSH passwords stored in memory or configuration are vulnerable.

**Attack Vectors:**
- Memory dumps containing passwords
- Config files with plaintext credentials
- Command line arguments visible in process list
- Environment variables leaked to child processes

**Mitigations:**

```rust
pub struct SecureCredentials {
    // Password is zeroed on drop
    password: Option<Zeroizing<String>>,
    // Key path only, key is loaded securely
    key_path: Option<PathBuf>,
}

impl Drop for SecureCredentials {
    fn drop(&mut self) {
        if let Some(ref mut password) = self.password {
            // Zero memory
            for byte in password.as_bytes_mut() {
                *byte = 0;
            }
        }
    }
}

// NEVER store passwords in config
pub struct SshConfig {
    host: String,
    user: String,
    port: u16,
    // Key-based auth only
    identity_file: Option<PathBuf>,
    // NO password field
}
```

### Threat: Session Hijacking

Active SSH sessions can be intercepted or hijacked.

**Attack Vectors:**
- Shared terminal sessions
- TTY file descriptor access
- Process injection (ptrace)
- SSH agent forwarding abuse

**Mitigations:**

```rust
pub struct SessionProtection {
    tty_permissions_checked: bool,
    agent_forwarding_disabled: bool,
    process_isolation: bool,
}

impl SessionProtection {
    pub fn setup(&mut self) -> Result<(), Error> {
        // Tighten TTY permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let tty = std::env::var("TTY").unwrap_or_else(|_| "/dev/tty".to_string());
            std::fs::set_permissions(&tty, std::fs::Permissions::from_mode(0o600))?;
        }
        
        // Disable agent forwarding by default
        self.agent_forwarding_disabled = true;
        
        Ok(())
    }
}
```

### Threat: Known Hosts Tampering

Modified known_hosts can enable MITM attacks.

**Attack Vectors:**
- Host key replacement
- Known_hosts file deletion
- Hash collision in hashed hosts
- DNS rebinding

**Mitigations:**

```rust
pub struct HostKeyVerification {
    strict_checking: bool,
    auto_reject_new_hosts: bool,
}

impl HostKeyVerification {
    pub fn verify(&self, host: &str, key: &str) -> Result<(), Error> {
        let known_hosts = self.load_known_hosts()?;
        
        match known_hosts.get(host) {
            Some(known_key) => {
                if known_key == key {
                    Ok(())
                } else {
                    tracing::error!(host, "Host key changed!");
                    Err(Error::HostKeyChanged { host: host.to_string() })
                }
            }
            None => {
                if self.auto_reject_new_hosts {
                    Err(Error::UnknownHost { host: host.to_string() })
                } else {
                    // Prompt user
                    if self.prompt_user(&format!("Accept new host key for {}?", host))? {
                        self.add_known_host(host, key)?;
                        Ok(())
                    } else {
                        Err(Error::HostKeyRejected)
                    }
                }
            }
        }
    }
}
```

---

## Clipboard Security

### Threat: Sensitive Data in Clipboard

Password managers clip passwords, SSH keys in clipboard, etc.

**Attack Vectors:**
- OSC 52 clipboard write from remote
- Clipboard history persistence
- Third-party clipboard managers
- Clipboard sync between devices

**Mitigations:**

```rust
pub struct ClipboardProtection {
    sensitive_patterns: Vec<Regex>,
    max_clipboard_age: Duration,
    clear_on_exit: bool,
}

impl ClipboardProtection {
    pub fn check_sensitive(&self, data: &str) -> bool {
        // Check for sensitive patterns
        for pattern in &self.sensitive_patterns {
            if pattern.is_match(data) {
                return true;
            }
        }
        
        // Heuristics
        data.len() > 50 && data.chars().all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=')
            // Looks like base64
    }
    
    pub fn clear_on_exit(&self) {
        if self.clear_on_exit {
            // Clear clipboard on exit
            self.clipboard.clear();
        }
    }
}
```

### Threat: Clipboard Hijacking

Malicious applications can read/modify clipboard.

**Attack Vectors:**
- Background clipboard monitoring
- Clipboard modification (phishing)
- Primary vs CLIPBOARD selection (X11)

**Mitigations:**

```rust
// Only read clipboard on explicit user action
pub async fn paste(&self) -> Result<String, Error> {
    // Verify user initiated (keyboard/mouse event)
    if !self.last_input_was_user_initiated() {
        return Err(Error::ClipboardNotUserInitiated);
    }
    
    self.clipboard.read()
}

// X11: Separate selections
#[cfg(target_os = "linux")]
pub enum Selection {
    Primary,  // Middle-click paste
    Clipboard, // Ctrl+V paste
}
```

---

## Network Security (SSH)

### Threat: Man-in-the-Middle

SSH sessions can be intercepted.

**Attack Vectors:**
- Host key spoofing
- DNS spoofing
- IP spoofing
- Certificate authority compromise

**Mitigations:**

See known_hosts checking above. Additionally:

```rust
pub struct SshClient {
    host_key_policy: HostKeyPolicy,
    cipher_preferences: Vec<String>,
}

impl SshClient {
    pub fn connect(&mut self, host: &str, config: SshConfig) -> Result<Session, Error> {
        // Enforce strong ciphers
        let session = Session::connect(&self.cipher_preferences)?;
        
        // Verify host key
        session.verify_host_key(host, &self.host_key_policy)?;
        
        Ok(session)
    }
}
```

### Threat: DNS Rebinding

Local network access via SSH tunnels.

**Attack Vectors:**
- Remote port forwarding to localhost
- Dynamic SOCKS proxy abuse
- X11 forwarding

**Mitigations:**

```rust
pub struct PortForwardingPolicy {
    allow_remote_to_local: bool,
    allow_local_to_remote: bool,
    allowed_remote_ports: Vec<u16>,  // Only allow forwarding to specific ports
}

impl Default for PortForwardingPolicy {
    fn default() -> Self {
        Self {
            allow_remote_to_local: false,  // Block by default
            allow_local_to_remote: true,
            allowed_remote_ports: vec![80, 443],  // HTTP(S) only via SOCKS proxy
        }
    }
}
```

### Threat: SSRF via Tunnel

SSH tunnels can access internal services.

**Attack Vectors:**
- Forward to internal services
- Access cloud metadata (169.254.169.254)
- Bypass firewall rules

**Mitigations:**

```rust
pub fn validate_tunnel_destination(&self, host: &str, port: u16) -> Result<(), Error> {
    // Block cloud metadata endpoints
    let blocked_hosts = [
        "169.254.169.254",  // AWS/Azure/GCP metadata
        "metadata.google.internal",
        "metadata.azure.internal",
    ];
    
    if blocked_hosts.contains(&host) {
        return Err(Error::BlockedTunnelDestination { host: host.to_string() });
    }
    
    // Block internal IP ranges
    if let Ok(ip) = host.parse::<IpAddr>() {
        if ip.is_loopback() || ip.is_link_local() {
            return Err(Error::BlockedTunnelDestination { host: host.to_string() });
        }
    }
    
    Ok(())
}
```

---

## File Access

### Threat: Path Traversal

Terminal features that access files can be abused.

**Attack Vectors:**
- OSC 7 working directory
- File drop/download paths
- Configuration file paths
- Log file paths

**Mitigations:**

```rust
pub fn sanitize_path(&self, path: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(path);
    
    // Canonicalize to resolve .. and symlinks
    let canonical = path.canonicalize()
        .map_err(|_| Error::InvalidPath { path: path.display().to_string() })?;
    
    // Verify it's under an allowed root
    for root in &self.config.allowed_roots {
        if canonical.starts_with(root.canonicalize()?) {
            return Ok(canonical);
        }
    }
    
    Err(Error::PathNotAllowed { path: path.display().to_string() })
}
```

### Threat: Sensitive File Access

Terminal may read paths containing secrets.

**Attack Vectors:**
- `~/.ssh/id_rsa` via file read
- `~/.pgpass` postgres password file
- `~/.netrc` network credentials
- Environment files (`.env`)

**Mitigations:**

```rust
pub const SENSITIVE_FILES: &[&str] = &[
    ".ssh/id_rsa",
    ".ssh/id_ed25519",
    ".pgpass",
    ".netrc",
    ".env",
    "credentials.json",
    "secrets.json",
];

pub fn validate_file_read(&self, path: &Path) -> Result<(), Error> {
    let path_str = path.to_string_lossy();
    
    for sensitive in SENSITIVE_FILES {
        if path_str.ends_with(sensitive) {
            tracing::warn!(path = %path.display(), "Attempted read of sensitive file");
            return Err(Error::SensitiveFileAccess { path: path.display().to_string() });
        }
    }
    
    Ok(())
}
```

### Threat: Config File Tampering

Configuration files can be modified by malicious processes.

**Attack Vectors:**
- Overwrite config with malicious values
- Inject code into config
- Symlink attacks

**Mitigations:**

```rust
pub fn load_config(&self) -> Result<Config, Error> {
    let path = self.config_path();
    let metadata = std::fs::metadata(&path)?;
    
    // Check permissions (owner read/write only on Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        if mode & 0o077 != 0 {
            tracing::warn!(mode = format!("{:o}", mode), "Insecure config permissions");
        }
    }
    
    // Check it's a regular file, not a symlink
    if metadata.is_symlink() {
        return Err(Error::ConfigIsSymlink);
    }
    
    // Parse and validate
    let contents = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&contents)?;
    config.validate()?;
    
    Ok(config)
}
```

---

## Dependency Security

### Threat: Supply Chain Attacks

Dependencies can contain malicious code.

**Attack Vectors:**
- Compromised crate
- Typosquatting
- Malicious maintainer
- Compromised build system

**Mitigations:**

1. **Dependency Audit**:
   ```bash
   # Run regularly
   cargo audit
   ```

2. **Lock File**:
   - Commit `Cargo.lock`
   - Verify checksums match
   - Use `cargo install --locked`

3. **Minimal Dependencies**:
   ```toml
   # Prefer standard library where possible
   [dependencies]
   thiserror = "1.0"    # Minimal error handling
   parking_lot = "0.12" # Minimal synchronization

   # Avoid heavy dependencies if possible
   # regex -> parse manually for simple cases
   # chrono -> time crate for simple timestamps
   ```

4. **Vendor Dependencies**:
   ```bash
   # Vendor for reproducible builds
   cargo vendor
   ```

### Threat: CVEs in Dependencies

Known vulnerabilities in dependencies.

**Mitigations:**

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "daily"
    open-pull-requests-limit: 10
    
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
```

```bash
# CI pipeline
cargo audit --deny warnings
cargo outdated --exit-code 1
```

### Threat: System Font Parsing

Renderer-side font discovery and rasterization increases attack surface because
font parsers consume bytes from system font directories.

**Attack Vectors:**
- malicious or corrupted font files installed on the host
- oversized font files intended to waste parse time or memory
- parser vulnerabilities in font-loading dependencies

**Mitigations:**
- treat system fonts as untrusted input at parse boundaries
- bound accepted font data sizes before parsing
- bound rasterized glyph dimensions before atlas allocation
- keep `cargo audit` in the regular verification loop for font-related dependencies

### Threat: License Compliance

Incompatible licenses can create legal issues.

**Mitigations:**

```bash
# Check licenses
cargo license
```

```toml
# Cargo.toml - reject incompatible licenses
[package.metadata.license]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib"]
deny = ["GPL-2.0", "GPL-3.0", "AGPL-3.0"]
```

---

## Persistence Security

### Threat: Session Recording

Terminal session recordings may contain secrets.

**Attack Vectors:**
- Passwords in command history
- API keys in output
- Session recordings stored insecurely

**Mitigations:**

```rust
pub struct SessionRecording {
    enabled: bool,
    exclude_patterns: Vec<Regex>,
    output_path: PathBuf,
}

impl SessionRecording {
    pub fn should_record(&self, line: &str) -> bool {
        // Don't record lines matching patterns
        for pattern in &self.exclude_patterns {
            if pattern.is_match(line) {
                return false;
            }
        }
        true
    }
}

// Default exclusions
pub fn default_exclude_patterns() -> Vec<Regex> {
    vec![
        Regex::new(r"(?i)password:\s*$").unwrap(),
        Regex::new(r"(?i)api[_-]?key\s*[=:]\s*\S+").unwrap(),
        Regex::new(r"(?i)secret[_-]?key\s*[=:]\s*\S+").unwrap(),
    ]
}
```

### Threat: Configuration Persistence

Saved configurations may contain secrets.

**Mitigations:**

```rust
// NEVER save passwords/keys to config
pub struct SavedConnection {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    // Password is NEVER saved
    // Key path is saved, but key itself is never in config
    pub identity_file: Option<PathBuf>,
}
```

---

## Security Checklist

### Release Security Review

- [ ] `cargo audit` passes with zero vulnerabilities
- [ ] No secrets in error messages
- [ ] No secrets in logs
- [ ] Clipboard write prompts configured
- [ ] OSC sequence filtering enabled
- [ ] Host key verification strict
- [ ] File access restrictions in place
- [ ] Dependency licenses reviewed
- [ ] Config file permissions checked
- [ ] Session recording excludes sensitive patterns

### Security Testing

```rust
#[test]
fn no_passwords_in_error_messages() {
    let error = PtyError::SpawnFailed {
        command: "ssh user:password@host".to_string(),
    };
    assert!(!error.to_string().contains("password"));
}

#[test]
fn clipboard_write_prompts_by_default() {
    let config = Config::default();
    assert!(config.clipboard_prompt);
}

#[test]
fn sensitive_files_are_blocked() {
    let security = Security::default();
    assert!(security.validate_file_read(Path::new("/home/user/.ssh/id_rsa")).is_err());
}
```

---

## Security Configuration

```toml
# Iris security defaults

[security]
# Require user confirmation for clipboard writes from remote
clipboard_prompt = true

# Block OSC sequences by default, whitelist explicitly
osc_whitelist_only = true

# Strict host key checking for SSH
strict_host_key_checking = true

# Block sensitive file reads
block_sensitive_files = true

# Clear clipboard on exit
clear_clipboard_on_exit = true

# Don't save passwords
save_passwords = false

# Session recording excludes
[security.recording_exclude]
patterns = [
    "password:\\s*$",
    "api.?key\\s*[=:]\\s*\\S+",
]

# File access
[security.file_access]
allowed_roots = ["~", "/tmp"]
blocked_files = [
    "~/.ssh/id_rsa",
    "~/.ssh/id_ed25519",
    "~/.pgpass",
    "~/.netrc",
]

# Network (SSH)
[security.ssh]
port_forwarding_remote_to_local = false
allowed_remote_ports = [80, 443]
block_metadata_endpoints = true
```

---

## Incident Response

### Security Vulnerability Found

1. **Do NOT commit fix with public issue reference**
2. Email security@herma.sh with details
3. Wait for CVE assignment
4. Patch in private branch
5. Coordinate disclosure timeline
6. Release with security advisory

### Log Retention

- Security events: 90 days minimum
- Failed authentication: 1 year
- Host key changes: Permanent

### User Notification

On security events:
- Pop-up for clipboard write requests
- Warning banner for host key changes
- Notification for suspicious OSC sequences
