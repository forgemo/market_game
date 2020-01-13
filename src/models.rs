use uuid::Uuid;
use std::time::Instant;
use std::collections::HashMap;
use std::cmp::Ordering;
use serde::{Serialize, Serializer};

#[derive(Debug, Serialize)]
pub enum ErrorType {
    AssetNotFound(Uuid),
    PortfolioNotFound(Uuid),
    OrderNotFound(Uuid),
    NotEnoughMatchingOrdersToImmediatelyFillBestOrder,
    CantLockAmountForBestOrder,
    CantSplitOrder,
    InsufficientFreeAmount,
    InsufficientLockedAmount,
    InvalidAssetId,
    InvalidState,
    NoLimitForBestOrder,
    QuantityCantBeZero,
    LimitCantBeZero,
    EngineWasTooBusy,
}

pub type EngineResult<T> = Result<T, ErrorType>;

#[derive(Copy, Clone, Debug)]
pub enum Event {
    Order(Order),
    CancelOrder(PortfolioId, OrderId, AssetId),
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OrderMode {
    Best,
    Limit(usize),
}

impl OrderMode {
    fn get_limit(&self) -> EngineResult<usize> {
        match self {
            OrderMode::Limit(limit) => Ok(*limit),
            OrderMode::Best => Err(ErrorType::NoLimitForBestOrder),
        }
    }
}


#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum OrderSide {
    Sell,
    Buy,
}

#[derive(Copy, Clone, Debug)]
pub struct Order{
    pub(crate) id: Uuid,
    pub(crate) asset: Uuid,
    pub(crate) mode: OrderMode,
    pub(crate) side: OrderSide,
    pub quantity: usize,
    expires: Instant,
    created_at: Instant,
    portfolio: Uuid,
}


impl Order {
    pub fn new(
        portfolio: PortfolioId,
        asset: AssetId,
        side: OrderSide,
        quantity: usize,
        mode: OrderMode,
        expires: Instant) -> EngineResult<Order> {

        if quantity ==  0 {
            return Err(ErrorType::QuantityCantBeZero);
        }
        match mode {
            OrderMode::Limit(l ) if l == 0 => Err(ErrorType::LimitCantBeZero)?,
            _ => {},
        }

        Ok(Order {
            id: Uuid::new_v4(),
            asset,
            side,
            quantity,
            mode,
            expires,
            portfolio,
            created_at: Instant::now()
        })
    }

    fn matches(&self, o: &Order) -> bool {
        let assets_matching = self.asset == o.asset;
        if !assets_matching {return  false};

        let sides_matching = match (self.side, o.side) {
            (OrderSide::Sell, OrderSide::Buy) => true,
            (OrderSide::Buy, OrderSide::Sell) => true,
            _ => false,
        };
        if !sides_matching {return  false};

        let modes_matching = match (self.mode, o.mode) {
            (OrderMode::Limit(_), OrderMode::Best) => true,
            (OrderMode::Best, OrderMode::Limit(_)) => true,
            (OrderMode::Best, OrderMode::Best) => false,
            (OrderMode::Limit(a), OrderMode::Limit(b)) => {
                match (self.side, o.side) {
                    (OrderSide::Sell, OrderSide::Buy) => a <= b,
                    (OrderSide::Buy, OrderSide::Sell) => a >= b,
                    _ => false,
                }
            }
        };

        return modes_matching;
    }

    fn split(&self, split_quantity: usize) -> EngineResult<(Order, Order)> {
        if split_quantity >= self.quantity {
            return Err(ErrorType::CantSplitOrder)
        };
        let mut left = self.clone();
        left.quantity = split_quantity;
        let mut right = self.clone();
        right.quantity = self.quantity - left.quantity;

        debug_assert_eq!(left.quantity + right.quantity, self.quantity);
        Ok((left, right))
    }


}

#[derive(Clone, Debug)]
pub struct Book {
    pub asset_id: AssetId,
    pub sell_orders: Vec<Order>,
    pub buy_orders: Vec<Order>,
}

impl Book {

    pub fn new(asset_id: AssetId) -> Book {
        Book {
            asset_id,
            sell_orders: vec![],
            buy_orders: vec![],
        }
    }
    fn add_order(&mut self, order: Order) -> EngineResult<()> {
        if order.mode == OrderMode::Best {
            return Err(ErrorType::NotEnoughMatchingOrdersToImmediatelyFillBestOrder);
        }
        match order.side {
            OrderSide::Sell => {
                self.sell_orders.push(order);
                self.sort_sell_orders();
            },
            OrderSide::Buy => {
                self.buy_orders.push(order);
                self.sort_buy_orders();
            },
        };
        Ok(())
    }

    fn remove_order(&mut self, id: Uuid) {
        self.sell_orders.retain(|sell| sell.id != id);
        self.buy_orders.retain(|buy| buy.id != id);
    }

    fn sort_buy_orders(&mut self) {
        self.buy_orders.sort_by(|a, b| Book::cmp_orders(a, b, false));
    }

    fn sort_sell_orders(&mut self) {
        self.buy_orders.sort_by(|a, b| Book::cmp_orders(a, b, false));
    }

    fn find_best_candidates_to_fill(&self, order: &Order) -> Vec<Order> {
        let other_side = match order.side {
            OrderSide::Sell => &self.buy_orders,
            OrderSide::Buy => &self.sell_orders,
        };

        let mut candidates :Vec<Order> = vec![];
        let mut fill_count = 0;
        for buy_order in other_side {
            if order.matches(&buy_order) {
                candidates.push(*buy_order);
                fill_count += buy_order.quantity;
            } else {
                break;
            }
            if fill_count >= order.quantity {
                break;
            }
        }
        return candidates;
    }

    fn get_order(&self, id: Uuid) -> EngineResult<&Order> {
        self.sell_orders.iter().find(|o| o.id == id)
            .or_else(|| self.buy_orders.iter().find(|o| o.id == id))
            .ok_or(ErrorType::OrderNotFound(id))
    }
    fn cmp_orders(a: &Order, b: & Order, revert_price_order: bool) -> Ordering {
        let mut order = match (a.mode, b.mode) {
            (OrderMode::Best, OrderMode::Best) => Ordering::Equal,
            (OrderMode::Best, OrderMode::Limit(_)) => Ordering::Less,
            (OrderMode::Limit(_), OrderMode::Best) => Ordering::Greater,
            (OrderMode::Limit(a), OrderMode::Limit(b)) => a.cmp(&b)
        };
        if revert_price_order {
            order.reverse();
        }
        if order == Ordering::Equal {
            order = a.created_at.cmp(&b.created_at);
        }
        order
    }
}


#[derive(Clone, Serialize)]
pub struct Asset {
    pub id: Uuid,
    pub name: String,
}

impl Asset {
    pub fn new(name: String) -> Asset {
        Asset {
            id: Uuid::new_v4(),
            name
        }
    }
}

#[derive(Clone)]
pub struct Market {
    pub bank_account: usize,
    pub portfolios: HashMap<Uuid, Portfolio>,
    pub assets: HashMap<Uuid, Asset>,
    pub books: HashMap<Uuid, Book>,
}

impl Market {

    pub fn new() -> Market {
        Market {
            bank_account: 0,
            portfolios: HashMap::new(),
            assets: HashMap::new(),
            books: HashMap::new(),
        }
    }

    fn get_order_book_mut(&mut self, asset_id: Uuid) -> EngineResult<&mut Book> {
        self.books.get_mut(&asset_id)
            .ok_or(ErrorType::AssetNotFound(asset_id))
    }

    pub fn get_order_book(&self, asset_id: Uuid) -> EngineResult<&Book> {
        self.books.get(&asset_id)
            .ok_or(ErrorType::AssetNotFound(asset_id))
    }

    pub fn get_asset(&self, asset_id: &Uuid) -> EngineResult<&Asset> {
        self.assets.get(&asset_id)
            .ok_or(ErrorType::AssetNotFound(asset_id.clone()))
    }

    fn bill_fee(&mut self, portfolio_id: Uuid, amount: usize) -> EngineResult<()>{
        self.get_portfolio_mut(portfolio_id)?
            .coins.spend_from_free_amount(amount)?;
        self.bank_account += amount;
        Ok(())
    }

    fn fill_order(&mut self, order: Order)  -> EngineResult<()> {

        let book = self.get_order_book_mut(order.asset)?;

        let mut filled_order = order;
        let mut candidates = book.find_best_candidates_to_fill(&order);
        if candidates.is_empty() {
            self.add_order(order, true)?;
        }else {
            let mut add_after_trade: Option<Order> = None;
            let fill_sum: usize = candidates.iter().map(|c|c.quantity).sum();
            if fill_sum > order.quantity {
                let (remainder, filled) = candidates.last().unwrap().split(fill_sum - order.quantity)?;
                add_after_trade = Some(remainder);
                candidates.pop();
                candidates.push(filled);
            } else if fill_sum < order.quantity {
                let (filled, remainder) = order.split(fill_sum)?;
                add_after_trade = Some(remainder);
                filled_order = filled;
            }
            self.process_trade(filled_order, candidates)?;
            if let Some(o) = add_after_trade {
                self.add_order(o, false)?;
            }
        }

        Ok(())
    }

    fn process_trade(&mut self, filled_order: Order, other_side: Vec<Order>) -> EngineResult<()> {
        let (use_locked_coins, use_locked_assets) = match filled_order.side {
            OrderSide::Buy => (false, true),
            OrderSide::Sell => (true, false),
        };

        for other in &other_side {
            debug_assert_eq!(filled_order.asset, other.asset);

            let price_per_asset = match (filled_order.side, filled_order.mode) {
                (_, OrderMode::Best) => other.mode.get_limit()?,
                (_, OrderMode::Limit(limit)) => limit,
            };

            let (buyer, seller) = match filled_order.side {
                OrderSide::Buy => (filled_order.portfolio, other.portfolio),
                OrderSide::Sell => (other.portfolio, filled_order.portfolio),
            };


            self.exchange(
                buyer,
                seller,
                filled_order.asset,
                other.quantity,
                price_per_asset,
                use_locked_coins,
                use_locked_assets,
            )?;

            self.remove_order(other.asset,other.id)?;
            //self.cancel_order(other.portfolio, other.id, other.asset);
        }
        self.remove_order(filled_order.asset, filled_order.id)?;
        //self.cancel_order(filled_order.portfolio, filled_order.id, filled_order.asset);

        Ok(())
    }

    fn exchange(&mut self,
                buyer: PortfolioId,
                seller: PortfolioId,
                asset_id: Uuid,
                asset_count: usize,
                price_per_asset: usize,
                use_locked_coins: bool,
                use_locked_assets: bool,
    ) -> EngineResult<()> {
        self.transfer_asset(
            seller,
            buyer,
            asset_id,
            asset_count,
            use_locked_assets
        )?;

        self.transfer_coins(
            buyer,
            seller,
            price_per_asset * asset_count,
            use_locked_coins,
        )?;

        Ok(())
    }

    fn transfer_asset(&mut self,
                      from: PortfolioId,
                      to: PortfolioId,
                      asset: AssetId,
                      amount: usize,
                      spend_locked_assets: bool
    ) -> EngineResult<()>{
        {
            let from_account = self.get_portfolio_mut(from)?
                .get_asset_account_mut(asset)?;
            if spend_locked_assets {
                from_account.spend_from_locked_amount(amount)?;
            } else {
                from_account.spend_from_free_amount(amount)?;
            }
        }
        {
            let to_account = self.get_portfolio_mut(to)?
                .get_asset_account_mut(asset)?;
            to_account.add(amount);
        }

        Ok(())
    }

    fn transfer_coins(&mut self,
                      from: PortfolioId,
                      to: PortfolioId,
                      amount: usize,
                      spend_locked_coins: bool
    ) -> EngineResult<()>{
        {
            let from_portfolio = self.get_portfolio_mut(from)?;
            if spend_locked_coins {
                from_portfolio.coins.spend_from_locked_amount(amount)?;
            } else {
                from_portfolio.coins.spend_from_free_amount(amount)?;
            }
        }
        {
            let to_portfolio = self.get_portfolio_mut(to)?;
            to_portfolio.coins.add(amount);
        }

        Ok(())
    }

    fn add_order(&mut self, order: Order, lock_amount: bool) -> EngineResult<()> {

        let portfolio = self.get_portfolio_mut(order.portfolio)?;
        if lock_amount {
            let lock_account = match order.side {
                OrderSide::Sell => portfolio.get_asset_account_mut(order.asset)?,
                OrderSide::Buy => &mut portfolio.coins,
            };
            let amount = match (order.side, order.mode) {
                (OrderSide::Sell, OrderMode::Limit(_)) => order.quantity,
                (OrderSide::Buy, OrderMode::Limit(limit)) => order.quantity * limit,
                _ => Err(ErrorType::CantLockAmountForBestOrder)?,
            };
            lock_account.lock_amount(amount)?
        }
        self.get_order_book_mut(order.asset)?.add_order(order)?;

        Ok(())
    }

    fn get_portfolio_mut(&mut self, portfolio_id: Uuid) -> EngineResult<&mut Portfolio> {
        self.portfolios.get_mut(&portfolio_id)
            .ok_or(ErrorType::PortfolioNotFound(portfolio_id))
    }

    pub fn get_portfolio(&self, portfolio_id: Uuid) -> EngineResult<&Portfolio> {
        self.portfolios.get(&portfolio_id)
            .ok_or(ErrorType::PortfolioNotFound(portfolio_id))
    }

    fn remove_order(&mut self, asset: Uuid, order: Uuid) -> EngineResult<()>{
        self.get_order_book_mut(asset)?.remove_order(order);
        Ok(())
    }

    fn cancel_order(&mut self, portfolio_id: Uuid, order_id: Uuid, asset_id: Uuid) ->  EngineResult<()> {
        let order = *self.get_order_book(asset_id)?.get_order(order_id)?;
        if order.asset != asset_id {
            return Err(ErrorType::InvalidAssetId);
        }
        {
            let portfolio = self.get_portfolio_mut(portfolio_id)?;
            match (order.side, order.mode) {
                (OrderSide::Sell, OrderMode::Limit(_)) => {
                    portfolio.get_asset_account_mut(order.asset)?
                        .unlock_amount(order.quantity)?;
                },
                (OrderSide::Buy, OrderMode::Limit(limit)) => {
                    portfolio.coins.unlock_amount(limit*order.quantity)?;
                },
                (_, OrderMode::Best) => panic!("A 'Best' order should not exist in the book."),
            }
        }
        self.get_order_book_mut(asset_id)?.remove_order(order_id);
        Ok(())
    }

}

pub struct Engine {
    pub market: Market,
}

impl Engine {

    pub fn new() -> Engine {
        Engine {
            market: Market::new()
        }
    }

    fn bill_fee_for(&mut self, event: Event) -> EngineResult<()> {
        let portfolio = match event {
            Event::Order(o) => o.portfolio,
            Event::CancelOrder(portfolio, _, _) => portfolio
        };
        self.market.bill_fee(portfolio, 1)
    }

    pub fn process(&mut self, event: Event) -> EngineResult<()> {
        println!("event -> {:?}", event);
        self.bill_fee_for(event)?;
        let snapshot = self.market.clone();
        let result = match event {
            Event::Order(o) => self.market.fill_order(o),
            Event::CancelOrder(portfolio, order, asset) => {
                self.market.cancel_order(portfolio, order, asset)
            }
        };
        if result.is_err() {
            self.market = snapshot;
        }
        result
    }
}

pub type AccountId = Uuid;
pub type AssetId = Uuid;
pub type OrderId = Uuid;
pub type PortfolioId = Uuid;


#[derive(Clone, Debug, Serialize)]
pub struct Portfolio {
    pub id: Uuid,
    pub coins: Account,
    pub assets: HashMap<AssetId, Account>
}

impl Portfolio {

    pub fn new(initial_coins: usize) -> Portfolio {
        Portfolio {
            id: Uuid::new_v4(),
            coins: Account::new(initial_coins),
            assets: HashMap::new(),
        }
    }
    pub fn get_asset_account_mut(&mut self, asset_id: Uuid) -> EngineResult<&mut Account> {
        self.assets.get_mut(&asset_id).ok_or(ErrorType::AssetNotFound(asset_id))
    }
}


#[derive(Clone, Debug, Serialize)]
pub struct Account {
    total_amount: usize,
    locked_amount: usize,
}

impl Account {

    pub(crate) fn new(initial_amount: usize) -> Account {
        Account {
            total_amount: initial_amount,
            locked_amount: 0,
        }
    }

    fn lock_amount(&mut self, amount_to_lock: usize) -> EngineResult<()> {
        if self.get_free_amount() >= amount_to_lock {
            self.locked_amount += amount_to_lock;
        } else {
            return Err(ErrorType::InsufficientFreeAmount);
        }
        debug_assert!(self.locked_amount <= self.total_amount);
        Ok(())
    }

    fn spend_from_locked_amount(&mut self, amount_to_spend: usize) -> EngineResult<()> {
        if self.locked_amount >= amount_to_spend {
            self.locked_amount -= amount_to_spend;
            debug_assert!(self.total_amount >= amount_to_spend);
            self.total_amount -= amount_to_spend;
        } else {
            return Err(ErrorType::InsufficientLockedAmount);
        }
        Ok(())
    }

    fn spend_from_free_amount(&mut self, amount_to_spend: usize) -> EngineResult<()> {
        if self.get_free_amount() >= amount_to_spend {
            self.total_amount -= amount_to_spend;
        } else {
            return Err(ErrorType::InsufficientFreeAmount);
        }
        Ok(())
    }

    fn unlock_amount(&mut self, amount_to_unlock: usize) -> EngineResult<()>{
        if self.locked_amount >= amount_to_unlock {
            self.locked_amount -= amount_to_unlock;
        } else {
            return Err(ErrorType::InsufficientLockedAmount);
        }
        Ok(())
    }

    pub fn add(&mut self, amount: usize) {
        self.total_amount += amount;
    }


    pub fn get_free_amount(&self) -> usize {
        self.total_amount - self.locked_amount
    }
}


