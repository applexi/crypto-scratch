# k-of-n-rss-mpc

A from-scratch simulation of secure multi-party computation (MPC) over k-of-n (where k = (n + 1)/2) replicated secret sharing (RSS). In this case, security guarantees throughout are that no k - 1 parties can learn any information about the secret(s).

The MPC system includes:
- Creation/deletion/MPC operations on arithmetic variables of size `u16`
- Multiplication, optimized with PRNG to reduce communication between parties
- Most significant bit (MSB) extraction, optimized with local computation to reduce binary adder calls
- Full and parallel prefix binary adders (for MSB extraction), optimized for reduced AND operations

> This is a learning project, not a security product. 
> Code (especially data structure representations) can be further optimized, but has not been done for the sake of learning/understanding.

## Usage

Example in [`src/main.rs`](src/main.rs).

```bash
cargo run   
cargo test  
```
