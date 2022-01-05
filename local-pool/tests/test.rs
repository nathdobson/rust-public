use local_pool::{run_until};
use std::time::Duration;
use futures::executor::block_on;

registry!{
    require status;
}

#[test]
fn test_foofofo() {
    REGISTRY.build();
    let n = block_on(run_until(async move {
        println!("S1");
        let handle2 = local_pool::spawn(async move {
            println!("Enter 1");
            return 2;
        });
        println!("S2");
        let handle3 = local_pool::spawn(async move {
            println!("Enter 2");
            return 3;
        });
        println!("S3");
        let x = handle2.await.unwrap();
        println!("S4");
        let y = handle3.await.unwrap();
        println!("S5");
        return x + y;
    }));
    assert_eq!(n, 5);
}