use crate::models::{Asset, EngineResult, Portfolio, Order, OrderSide, OrderMode, Event, ErrorType};
use crate::game::{Game, PublicBook};
use rocket::{State, Request, response, Response};
use rocket_contrib::json::{Json};
use rocket_contrib::uuid::Uuid;
use rocket::http::ContentType;
use std::time::{Instant, Duration};
use std::ops::Add;
use rocket::response::Responder;
use std::io::Cursor;

#[get("/portfolio/<id>")]
fn get_portfolio(id: Uuid, game: State<Game>) -> EngineResult<Json<Portfolio>> {
    return game.read_engine()?.market.get_portfolio(*id).map(|p|Json(p.clone()))
}

#[get("/asset")]
fn get_assets(game: State<Game>) -> EngineResult<Json<Assets>>{
    Ok(Json(Assets{
        assets: game.read_engine()?.market.assets.values().map(|a|a.clone()).collect::<Vec<_>>()
    }))
}

#[get("/book")]
fn get_books(game: State<Game>) -> EngineResult<Json<Books>>{
    Ok(Json(Books{
        books: game.get_public_books()?
    }))
}

#[get("/book/<asset>")]
fn get_book(asset: Uuid, game: State<Game>) -> EngineResult<Json<PublicBook>>{
    Ok(Json(game.get_public_book_for(*asset)?))
}

#[get("/asset/<id>")]
fn get_asset(id: Uuid, game: State<Game>) -> EngineResult<Json<Asset>> {
    return game.read_engine()?.market.assets.get(&id)
        .map(|a|Json(a.clone())).ok_or(ErrorType::AssetNotFound(*id))
}


#[delete("/portfolio/<portfolio>/asset/<asset>/order/<order>")]
fn cancel_order(portfolio: Uuid, asset: Uuid, order: Uuid, game: State<Game>) -> EngineResult<()> {
    return game.write_engine()?.process(Event::CancelOrder(*portfolio,*order,*asset))
}



#[post("/portfolio/<portfolio>/asset/<asset>/sell", data="<data>")]
fn sell_order(portfolio: Uuid, asset: Uuid, data: Json<OrderPlacement>, game: State<Game>, )
              -> EngineResult<Json<uuid::Uuid>> {
    let o =  Order::new(
        *portfolio,
        *asset,
        OrderSide::Sell,
        data.quantity,
        data.mode,
        Instant::now().add(Duration::from_secs(24*60*60))
    )?;
    game.write_engine()?.process(Event::Order(o))?;
    return Ok(Json(o.id));
}

#[post("/portfolio/<portfolio>/asset/<asset>/buy", data="<data>")]
fn buy_order(portfolio: Uuid, asset: Uuid, data: Json<OrderPlacement>, game: State<Game>, )
              -> EngineResult<Json<uuid::Uuid>> {
    let o =  Order::new(
        *portfolio,
        *asset,
        OrderSide::Buy,
        data.quantity,
        data.mode,
        Instant::now().add(Duration::from_secs(24*60*60))
    )?;
    game.write_engine()?.process(Event::Order(o))?;
    return Ok(Json(o.id));
}

#[derive(Serialize, Deserialize)]
pub struct OrderPlacement {
    quantity: usize,
    mode: OrderMode,
}



pub fn start_server(game: Game) {

    let aaa = OrderPlacement {
        quantity: 2,
        mode: OrderMode::Limit(3),
    };
    let encoded = serde_json::to_string(&aaa).unwrap();
    println!("{}", encoded);
    rocket::ignite().mount("/", routes![
        get_portfolio,
        get_asset,
        get_assets,
        sell_order,
        buy_order,
        cancel_order,
        get_book,
        get_books,
    ]).manage(game).launch();
}

#[derive(Serialize)]
struct Assets {
    assets: Vec<Asset>
}

#[derive(Serialize)]
struct Books {
    books: Vec<PublicBook>
}


impl Responder<'_> for ErrorType {
    fn respond_to(self, _: &Request) -> response::Result<'static> {
        Response::build()
            .sized_body(Cursor::new(format!("{:?}", self)))
            .header(ContentType::new("text", "text"))
            .status(rocket::http::Status::BadRequest)
            .ok()
    }
}
