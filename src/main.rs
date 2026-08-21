use k_of_n_rss_mpc::{error::Error, full_adder, mpc::MPC, parallel_prefix, sharing::Arithmetic};
use rand::{TryRng, rngs::SysRng};

/// Feel free to edit to test
fn main() -> Result<(), Error>{
    // 3-of-5 RSS scheme
    let (k, n) = (3, 5);
    let mut rng = SysRng;
    let mut mpc = MPC::new(k, n);
    mpc.set_prng()?;

    // Define arithmetic variables "a" and "b", where a and b are random u16s
    let (name_a, name_b) = ("a".to_string(), "b".to_string());
    let a = rng.try_next_u32().map(|x| x as u16).map_err(|_| Error::Rng)?;
    let b = rng.try_next_u32().map(|x| x as u16).map_err(|_| Error::Rng)?;
    println!("Value a: {a}, Value b: {b}");

    // Secret share a and b using mpc to get [a] and [b]
    mpc.new_secret(&name_a, a)?;
    mpc.new_secret(&name_b, b)?;

    // Add [a] and [b] to create [c] 
    let name_c = "c".to_string();
    mpc.add(&name_a, &name_b, &name_c)?;
    let sharing_c = mpc.open_var(&name_c)?; 
    println!("Sharing c: {sharing_c}, Actual c: {:?}", a.wrapping_add(b));

    // Multiply [a] and [b] to create [d]
    let name_d = "d".to_string();
    mpc.mult(&name_a, &name_b, &name_d)?;
    let sharing_d = mpc.open_var(&name_d)?;
    println!("Sharing d: {sharing_d}, Actual d: {:?}", a.wrapping_mul(b));

    // Get and open the msb of [a] via a full adder
    let msb_a_full = mpc.get_msb(&name_a, full_adder)?;
    let msb_a_ppa = mpc.get_msb(&name_a, parallel_prefix)?;
    let Some(msb_a) = Arithmetic::to_binary(a).last().cloned() else {
        return Err(Error::String("Variable a has no msb".to_string()))
    };
    println!("MSB via full: {msb_a_full}, MSB via ppa: {msb_a_ppa}, Actual MSB: {msb_a}");
    Ok(())
}
