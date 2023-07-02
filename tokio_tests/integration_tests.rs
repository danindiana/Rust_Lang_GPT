use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::test]
async fn test_connection_establishment() {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await.expect("Should connect");
    stream.write_all(b"Hello").await.expect("Should write");
    let mut buffer = [0; 5];
    stream.read_exact(&mut buffer).await.expect("Should read");
    assert_eq!(buffer, b"Hello");
}

#[tokio::test]
async fn test_echo_functionality() {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    let msg = b"Hello World!";
    stream.write_all(msg).await.unwrap();
    
    let mut buffer = vec![0; msg.len()];
    stream.read_exact(&mut buffer).await.unwrap();
    
    assert_eq!(buffer, msg);
}

#[tokio::test]
async fn test_large_data_echo() {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    let msg = vec![1; 1024 * 1024]; // 1MB of data
    stream.write_all(&msg).await.unwrap();
    
    let mut buffer = vec![0; msg.len()];
    stream.read_exact(&mut buffer).await.unwrap();
    
    assert_eq!(buffer, msg);
}

#[tokio::test]
async fn test_concurrent_connections() {
    let mut handles = Vec::new();

    for _ in 0..10 {
        let handle = tokio::spawn(async {
            let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
            let msg = b"concurrent";
            stream.write_all(msg).await.unwrap();
            
            let mut buffer = vec![0; msg.len()];
            stream.read_exact(&mut buffer).await.unwrap();
            
            assert_eq!(buffer, msg);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_connection_termination() {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    let msg = b"terminate";
    stream.write_all(msg).await.unwrap();
    
    stream.shutdown().await.unwrap();
    let mut buffer = vec![0; 32];
    let size = stream.read(&mut buffer).await.unwrap();
    
    assert_eq!(size, 0); // No data should be read since the connection is terminated
}
