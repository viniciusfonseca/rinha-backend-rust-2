use anyhow::Error;

pub async fn consulta_saldo(id_cliente: i32) -> (i32, i32) {

    let b = reqwest::get(format!("unix:/tmp/sockets/api03.sock/c/{id_cliente}").as_str()).await
        .expect("error getting saldo")
        .text()
        .await.unwrap();
    let mut s = b.split(",");

    (
        s.nth(0).unwrap().parse::<i32>().unwrap(),
        s.nth(1).unwrap().parse::<i32>().unwrap()
    )
}

pub async fn movimenta_saldo(id_cliente: i32, tipo: &String, valor: i32) -> Result<(i32, i32), anyhow::Error> {
    let b = reqwest::get(format!("unix:/tmp/sockets/api03.sock/c/{id_cliente}/{tipo}/{valor}").as_str()).await
        .expect("error getting movimento")
        .text()
        .await.unwrap();

        let s = b.split(",").collect::<Vec<&str>>();
        if s.len() == 1 {
            Err(Error::msg(""))
        }
        else {
            Ok((
                s.get(0).unwrap().parse::<i32>().unwrap(),
                s.get(1).unwrap().parse::<i32>().unwrap()
            ))
        }
}