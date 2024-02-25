use tokio::{fs::{File, OpenOptions}, io::{AsyncReadExt, AsyncSeekExt}};

// {tx_id},{value},{updated_value},{datetime_rfc3339},{tipo},{descricao}
pub type AtomicLog = (i32, i32, i32, String, String, String);

#[derive(Debug)]
pub struct AtomicFd {
    logs: File,
    log_size: usize
}

impl AtomicFd {
    pub async fn new(id: usize, log_size: usize) -> AtomicFd {
        AtomicFd {
            logs: OpenOptions::new()
                .create(true)
                .read(true)
                .write(false)
                .open(format!("/tmp/{id}.log")).await.unwrap(),
            log_size
        }
    }

    pub async fn get_logs(&mut self, max: usize) -> Vec<AtomicLog> {
        let buffer_size = self.log_size * max;
        _ = self.logs.seek(std::io::SeekFrom::End(0)).await;
        let cursor_target = -(TryInto::<i64>::try_into(buffer_size + 1)).unwrap();
        if self.logs.seek(std::io::SeekFrom::Current(cursor_target)).await.is_err() {
            _ = self.logs.seek(std::io::SeekFrom::Start(0)).await;
        }
        let mut buf = vec![0u8; buffer_size];
        let bytes_read = self.logs.read(&mut buf).await.unwrap();
        if bytes_read == 0 {
            return Vec::new()
        }
        let lines = String::from_utf8(buf).unwrap();
        let lines = lines.trim_matches(char::from(0x0A)).split("\n");
        let mut r = Vec::new();
        for line in lines {
            let split = line.split(",").collect::<Vec<&str>>();
            let txid = match split.get(0).unwrap().trim_matches(char::from(0)).parse::<i32>() {
                Ok(i) => i,
                Err(_) => {
                    println!("error parsing txid. line: {line}, bytes_read: {bytes_read}, buf: {:?}", line.as_bytes());
                    continue;
                }
            };
            r.push((
                txid,
                split.get(1).unwrap().parse::<i32>().unwrap(),
                split.get(2).unwrap().parse::<i32>().unwrap(),
                split.get(3).unwrap().to_string(),
                split.get(4).unwrap().to_string(),
                split.get(5).unwrap().to_string().trim_end().to_string(),
            ))
        }
        r
    }
}