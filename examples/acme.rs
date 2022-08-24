use anyhow::bail;
use monoio_gateway::init_env;
use monoio_gateway_core::{
    acme::{lets_encrypt::GenericAcme, Acme, Acmed},
    error::GError,
};

#[monoio::main(timer_enabled = true)]
async fn main() -> Result<(), GError> {
    init_env();

    let server_name = "monoio-gateway.kingtous.cn";
    let location = server_name.get_acme_path()?;
    let acme = GenericAcme::new_lets_encrypt_staging(server_name.to_string());
    match acme.acme("me@kingtous.cn".to_string()).await {
        Ok(_cert) => {
            println!("get cert, location: {:?}", location);
        }
        Err(err) => {
            bail!("{}", err)
        }
    }

    Ok(())
}
