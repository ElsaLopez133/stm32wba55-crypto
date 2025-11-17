
# stm32wba55-crypto

Minimal Rust + STM32WBA55 example project.

## Setup

1. **Build device crate locally**  
   Follow the steps in [stm32-rs: Generating device crates](https://github.com/stm32-rs/stm32-rs/tree/master?tab=readme-ov-file#generating-device-crates--building-locally).  
   Then update this project `config.toml` to point to your locally built crate.

2. **Install ST-LINK tools (Arch Linux)**  
   ```bash
   sudo pacman -S stlink
   ```
3. **Run the example**

cargo run --bin blink
