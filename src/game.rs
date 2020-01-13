use crate::models::{Engine, Portfolio, PortfolioId, AssetId, Asset, Account, Book, EngineResult, ErrorType, OrderMode, OrderSide, Order};
use uuid::Uuid;
use std::sync::{Arc, RwLock, RwLockWriteGuard, RwLockReadGuard};

pub struct Game {
    pub engine: Arc<RwLock<Engine>>,
}


impl Game {
    pub fn new() -> Game  {
        Game {
            engine: Arc::new(RwLock::new(Engine::new()))
        }
    }

    pub fn read_engine(&self) -> EngineResult<RwLockReadGuard<Engine>> {
        self.engine.read().map_err(|_| ErrorType::EngineWasTooBusy)
    }

    pub fn write_engine(&self) -> EngineResult<RwLockWriteGuard<Engine>> {
        self.engine.write().map_err(|_| ErrorType::EngineWasTooBusy)
    }

    pub fn create_portfolio(&mut self, initial_coins: usize) -> PortfolioId {
        let mut portfolio = Portfolio::new(initial_coins);
        let mut engine = self.write_engine().unwrap();
        engine.market.assets.values().for_each(|asset|{
           portfolio.assets.insert(asset.id, Account::new(0));
        });
        let id = portfolio.id;
        engine.market.portfolios.insert(id,portfolio );
        return id;
    }

    pub fn create_asset(&mut self, name: String) -> AssetId {
        let asset = Asset::new(name);
        let id = asset.id;
        let mut engine = self.write_engine().unwrap();
        engine.market.assets.insert(id, asset);
        engine.market.portfolios.values_mut().for_each(|portfolio|{
            portfolio.assets.insert(id, Account::new(0));
        });
        engine.market.books.insert(id, Book::new(id));
        return id;
    }

    pub fn set_asset_amount(&mut self, portfolio: Uuid,  asset: Uuid, amount: usize) {
        self.write_engine().unwrap().market.portfolios.get_mut(&portfolio).unwrap()
            .assets.get_mut(&asset).unwrap().add(amount);
    }

    pub fn get_public_books(&self) -> EngineResult<Vec<PublicBook>> {
        let engine = self.read_engine()?;
        engine.market.assets.values().map(|asset| {
            engine.market.get_order_book(asset.id.clone())
                .map(|b| PublicBook::from_book(asset.clone(), b))
        }).collect()
    }

    pub fn get_public_book_for(&self, asset_id: Uuid) -> EngineResult<PublicBook> {
        let engine = self.read_engine()?;
        let book = engine.market.get_order_book(asset_id)?;
        let asset = engine.market.get_asset(&asset_id)?;
        Ok(PublicBook::from_book(asset.clone(), book))
    }

}

#[derive(Serialize)]
pub struct  PublicBook {
    asset: Asset,
    sell: Vec<PublicOrder>,
    buy: Vec<PublicOrder>,
}

impl PublicBook {
    pub fn from_book(asset: Asset, book: &Book) -> PublicBook {
        PublicBook {
            asset,
            sell: book.sell_orders.iter().map(|o|PublicOrder::from(o)).collect(),
            buy: book.buy_orders.iter().map(|o|PublicOrder::from(o)).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct PublicOrder {
    pub asset: Uuid,
    pub mode: OrderMode,
    pub side: OrderSide,
    pub quantity: usize,
}

impl PublicOrder {
    pub fn from(order: &Order) -> PublicOrder {
        PublicOrder {
            asset: order.asset,
            mode: order.mode,
            side: order.side,
            quantity: order.quantity,
        }
    }
}

