use consulrs::client::{ConsulClient, ConsulClientSettingsBuilder};
use consulrs::error::ClientError;
use consulrs::catalog::services;
use consulrs::service::register;
use consulrs::api::service::requests::RegisterServiceRequestBuilder;

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    let client = ConsulClient::new(
        ConsulClientSettingsBuilder::default()
            .address("http://192.168.10.42:8500")
            .build()
            .unwrap()
    )?;
    let services = services(&client, None).await?; 
    for service in services.response {
        println!("{:?}", service);
    }
    let response = register(&client, "test", 
        Some(RegisterServiceRequestBuilder::default()
            .name("test")
            .address("127.0.0.1")
            .port(8080_u64)
            .kind("consulrs")
            .tags(vec!["consulrs".to_string()]))
        ).await?;
    println!("{:?}", response);

    Ok(())

}
