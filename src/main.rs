struct User {
    username: String,
    count: u32,
}

impl User {
    fn printName(&self){
        println!("{}",self.username);
    }
}

fn build_user(count: u32) -> User
{
    let original_user = User {
        username: String::from("test"),
        count,
    };
    
    User {
        count: 42,
        ..original_user
    }
}

fn main() {
    println!("Hello, world!");
    let taishou_a = User {
    username: String::from("myname"),
    count: 4,
    };
    
    let taishou_b = build_user(1234);
    
    println!("count:{} username:{}", taishou_a.count, taishou_a.username);
    
    println!("count:{} username:{}", taishou_b.count, taishou_b.username);

    taishou_a.printName();

    let mut x = Some(5);
    if 1 == 1
    {
        x = Some(1234);
    }
    else {
        x = None;
    }



}
