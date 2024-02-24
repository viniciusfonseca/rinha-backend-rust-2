use tokio::{fs::{File, OpenOptions}, io::{AsyncReadExt, AsyncSeekExt}};

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
            value: OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .open(format!("/tmp/{id}.a")).await.unwrap(),
            logs: OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .open(format!("/tmp/{id}.log")).await.unwrap(),
            log_size
        }
    }

    pub async fn get_value(&mut self) -> i32 {
        _ = self.value.rewind().await;
        let mut buf = String::new();
        _ = self.value.read_to_string(&mut buf).await;
        buf.trim_matches(char::from(0)).parse::<i32>().unwrap()
    }

    pub async fn get_logs(&mut self, max: usize) -> Vec<AtomicLog> {
        let buffer_size = self.log_size * max;
        _ = self.logs.seek(std::io::SeekFrom::End(0)).await;
        let cursor_target = -(TryInto::<i64>::try_into(buffer_size + 1)).unwrap();
        if self.logs.seek(std::io::SeekFrom::Current(cursor_target)).await.is_err() {
            _ = self.logs.seek(std::io::SeekFrom::Start(0)).await;
        }
        let mut buf = vec![0 as u8; buffer_size];
        if self.logs.read(&mut buf).await.unwrap() == 0 {
            return Vec::new()
        }
        let lines = String::from_utf8(buf).unwrap();
        let lines = lines.trim_matches(char::from(0x0A)).split("\n");
        let mut r = Vec::new();
        for line in lines {
            let split = line.split(",").collect::<Vec<&str>>();
            r.push((
                split.get(0).unwrap().trim_matches(char::from(0)).parse::<i32>().unwrap(),
                split.get(1).unwrap().parse::<i32>().unwrap(),
                split.get(2).unwrap().to_string(),
                split.get(3).unwrap().to_string(),
                split.get(4).unwrap().to_string().trim_end().to_string(),
            ))
        }
        r
    }
}