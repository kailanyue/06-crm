use anyhow::Result;
use crm::pd::User;
use prost::Message;

fn main() -> Result<()> {
    let user = User::new(1, "John", "X9Ejv@example.com");
    let encoded = user.encode_to_vec();
    println!("user: {:?} encoded: {:?}", user, encoded);

    let decoded = User::decode(&encoded[..])?;
    println!("decoded: {:?}", decoded,);
    Ok(())
}
