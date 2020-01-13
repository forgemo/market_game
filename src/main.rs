use market_game::models::*;
use std::time::{Instant, Duration};
use std::ops::Add;
use market_game::game::Game;
use market_game::server::start_server;


fn main() {
    let mut game = Game::new() ;
    let p1 = game.create_portfolio(1000);
    let p2 = game.create_portfolio(1000);
    let a1 = game.create_asset("a1_name".to_string());
    let a1 = game.create_asset("a2_name".to_string());
    //let a2 = game.create_asset("a2_name".to_string());
    game.set_asset_amount(p1, a1, 100);
    game.set_asset_amount(p2, a1, 100);

    println!("--------------");
    println!("{:?}", game.read_engine().unwrap().market.books.get(&a1));
    println!("bank: {:?}", game.read_engine().unwrap().market.bank_account);
    println!("port {:?}", game.read_engine().unwrap().market.portfolios.get(&p1).unwrap() );
    println!("port {:?}", game.read_engine().unwrap().market.portfolios.get(&p2).unwrap() );

    // sell order
    let o1 = Order::new(
        p1,
        a1,
        OrderSide::Buy,
        10,
        OrderMode::Limit(1),
        Instant::now().add(Duration::from_secs(24*60*60))
    ).unwrap();

    let r1 = game.engine.write().unwrap().process(Event::Order(o1));
    println!("resuult => {:?}", r1);

    println!("--------------");
    println!("{:?}", game.read_engine().unwrap().market.books.get(&a1));
    println!("bank: {:?}", game.read_engine().unwrap().market.bank_account);
    println!("port {:?}", game.read_engine().unwrap().market.portfolios.get(&p1).unwrap() );
    println!("port {:?}", game.read_engine().unwrap().market.portfolios.get(&p2).unwrap() );


    // buy order

    let o2 = Order::new(
        p2,
        a1,
        OrderSide::Sell,
        5,
        OrderMode::Best,
        Instant::now().add(Duration::from_secs(30*24*60*60))
    ).unwrap();
    let r2 = game.write_engine().unwrap().process(Event::Order(o2));

    println!("result => {:?}", r2);
    println!("--------------");
    println!("{:?}", game.read_engine().unwrap().market.books.get(&a1));
    println!("bank: {:?}", game.read_engine().unwrap().market.bank_account);
    println!("port {:?}", game.read_engine().unwrap().market.portfolios.get(&p1).unwrap() );
    println!("port {:?}", game.read_engine().unwrap().market.portfolios.get(&p2).unwrap() );

    start_server(game);
}


