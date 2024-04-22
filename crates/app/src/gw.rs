use base::state::{ReqMessage, RspMessage};
use base::*;
use clap::Parser;
use log::{error, info};
use rust_share::gateway::executor;
use rust_share::gateway::fronts;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

#[derive(Parser)]
#[clap()]
struct Opts {
    config: String,
}

#[tokio::main]
async fn main() {
    init_logger();
    let opts: Opts = Opts::parse();
    let conf = GatewayConfig::load(&opts.config).unwrap();
    let mut ve = vec![];
    for e in conf.executors.into_iter().filter(|x| x.enabled) {
        info!("{:?}", e);
        let (tx, rx) = oneshot::channel::<i32>();
        ve.push(rx);
        tokio::spawn(async move {
            let mut ex = executor::Executor::new();
            if e.r#type == "ctp_futures" {
                let cmc = ctp_futures::route::new_md_cache(&e.md_account);
                for ca in e.accounts.iter() {
                    let cmc = Arc::clone(&cmc);
                    let ca = ca.clone();
                    let (tx, rx) = mpsc::channel::<(
                        ReqMessage,
                        tokio::sync::oneshot::Sender<RspMessage>,
                    )>(1000);
                    ex.sorted_accounts.push(executor::TraderHandle {
                        account: ca.account.clone(),
                        tx,
                    });
                    tokio::spawn(async move {
                        if let Err(e) = ctp_futures::route::run_trader(ca, cmc, rx).await {
                            error!("ctp_futures trader exit {e}");
                        }
                    });
                }
            }
            if e.r#type == "tora_stock" {
                let cmc = tora_stock::route::new_md_cache(&e.md_account);
                for ca in e.accounts.iter() {
                    let cmc = Arc::clone(&cmc);
                    let ca = ca.clone();
                    let (tx, rx) = mpsc::channel::<(
                        ReqMessage,
                        tokio::sync::oneshot::Sender<RspMessage>,
                    )>(1000);
                    ex.sorted_accounts.push(executor::TraderHandle {
                        account: ca.account.clone(),
                        tx,
                    });
                    tokio::spawn(async move {
                        if let Err(e) = tora_stock::route::run_trader(ca, cmc, rx).await {
                            error!("tora_stock trader exit {e}");
                        }
                    });
                }
            }
            fronts::http::serve(e.clone(), Arc::new(Mutex::new(ex))).await;
            let _ = tx.send(1);
        });
    }
    for rx in ve.into_iter() {
        let _ = rx.await;
    }
}
