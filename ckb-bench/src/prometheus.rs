use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::thread::sleep;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use regex::Regex;
use crossbeam_channel::{Receiver};

#[derive(Debug, Serialize,Deserialize,Clone)]
pub struct MemoryUsageReport {
    pub(crate) ckb_sys_mem_process_rss_mb: Vec<usize>,
    pub(crate) ckb_sys_mem_process_vms_mb: Vec<usize>,
    pub(crate) timestamp: Vec<u128>,
}

pub struct MemoryUsageClient {
    url: String,
    client: Client,
}

impl MemoryUsageClient {
    pub fn new(url: String) -> Self {
        Self { url, client: Client::new() }
    }

    pub fn get_memory_usage(
        &self,
        log_duration: u64,
        stop_recv:Receiver<bool>
    ) -> MemoryUsageReport {
        let mut ckb_sys_mem_process_rss = vec![];
        let mut ckb_sys_mem_process_vms = vec![];
        let mut timestamp = vec![];

        loop {
            let response = self.client.get(self.url.as_str()).send().unwrap();

            let body = response.text().unwrap();
            let rss_re = Regex::new(r#"(?m)ckb_sys_mem_process\{type="rss"\} (\d+)"#).unwrap();
            let vms_re = Regex::new(r#"(?m)ckb_sys_mem_process\{type="vms"\} (\d+)"#).unwrap();
            for cap in rss_re.captures_iter(body.as_str()) {
                let value = &cap[1];
                let number: Result<usize, _> = value.parse();
                match number {
                    Ok(number) => {
                        ckb_sys_mem_process_rss.push(number / 1024 /1024 as usize)
                    }
                    Err(err1) => {
                        println!("rss is empty:{}",err1);
                        ckb_sys_mem_process_rss.push(0)
                    }
                }
            }
            for cap in vms_re.captures_iter(body.as_str()) {
                let value = &cap[1];
                let number: Result<usize, _> = value.parse();
                match number {
                    Ok(number) => {
                        ckb_sys_mem_process_vms.push(number / 1024 /1024 as usize)
                    }
                    Err(err) => {
                        println!("vms is empty:{}",err);
                        ckb_sys_mem_process_vms.push(0)
                    }
                }
            }
            timestamp.push(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());


            sleep(Duration::from_secs(log_duration));
            match stop_recv.try_recv() {
                Ok(_) => {break;}
                Err(_) => {continue}
            }
        }

        return MemoryUsageReport {
            ckb_sys_mem_process_rss_mb:ckb_sys_mem_process_rss,
            ckb_sys_mem_process_vms_mb:ckb_sys_mem_process_vms,
            timestamp,
        };
    }
}


#[test]
fn test1() {
    let client = MemoryUsageClient::new("http://18.162.180.86:8100".into());
    let ret = client.get_memory_usage(Duration::from_secs(3).as_secs(), Duration::from_secs(10));
    println!("{:?}", ret)
}