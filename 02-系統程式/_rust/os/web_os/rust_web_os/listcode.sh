find . -type f \( -name "*.rs" -o -name "*.ld" -o -name "*.toml" \) \
  -exec cat {} +
