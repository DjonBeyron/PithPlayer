fn main() {
    let token = std::env::var("PITH_CEV_TOKEN").unwrap_or_default();
    let cev = pith_cev::Cev::new(&token).expect("нужен PITH_CEV_TOKEN");
    println!("{:?}", cev.check());
}
