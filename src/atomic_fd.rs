use tokio::{fs::File, io::{AsyncReadExt, AsyncSeekExt}};

// {tx_id},{updated_value},{datetime_rfc3339},{tipo},{descricao}
pub type AtomicLog = (i32, i32, String, String, String);
pub struct AtomicFd {
    value: File,
    logs: File,
    log_size: usize
}

impl AtomicFd {
    pub async fn new(id: usize, log_size: usize) -> AtomicFd {
        AtomicFd {
            value: File::open(format!("/tmp/{id}.a")).await.unwrap(),
            logs: File::open(format!("/tmp/{id}.log")).await.unwrap(),
            log_size
        }
    }

    pub async fn get_value(&mut self) -> i32 {
        let r = self.value.read_i32().await.unwrap();
        _ = self.value.seek(std::io::SeekFrom::Start(0)).await;
        r
    }

    pub async fn get_logs(&mut self, max: usize) -> Vec<AtomicLog> {
        let buffer_size = self.log_size * max;
        let seek_res = self.value.seek(std::io::SeekFrom::End(buffer_size.try_into().unwrap() + 1)).await;
        if seek_res.is_err() {
            _ = self.value.seek(std::io::SeekFrom::Start(0)).await;
        }
        let mut buf = vec![0 as u8; buffer_size];
        self.value.read_buf(&mut buf).await.unwrap();
        let lines = String::from_utf8(buf).unwrap();
        let lines = lines.split("\n");
        let r = Vec::new();
        for line in lines {
            let split = line.split(",");
            r.push((
                split.get(0).parse::<i32>().unwrap()
                split.get(1).parse::<i32>().unwrap(),
                split.get(2).to_string(),
                split.get(3).to_string(),
                split.get(4).to_string(),
            ))
        }
        r
    }
}