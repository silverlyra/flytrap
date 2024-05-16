use std::env;

use flytrap::api::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("FLY_API_TOKEN")?;
    let client = Client::new(token);

    let apps = client.apps("personal").await?;
    for app in apps {
        println!("app {}:", app.name);

        let machines = client.machines(&app.name).await?;
        for m in machines {
            let up = if m.is_ready() { " (up)" } else { "" };
            println!("  {:<24} in {}: {:?}{}", m.name, m.location, m.state, up);
        }
    }

    Ok(())
}
