use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct Order {
    pub price: Decimal,
    pub size: Decimal,
}

#[derive(Debug, Clone)]
pub struct Side {
    vec: Vec<Order>,
    cap: usize,
    rev: bool,
}

#[derive(Debug, Clone)]
pub struct Book {
    pub bids: Side,
    pub asks: Side,
}

impl Side {
    fn new(cap: usize, rev: bool) -> Self {
        Self {
            vec: Vec::with_capacity(cap),
            cap,
            rev,
        }
    }
    
    pub fn orders(&self) -> &Vec<Order> {
        &self.vec
    }

    pub fn shot_update(&mut self, orders: Vec<Order>) {
        self.vec = orders;
    }

    pub fn diff_update(&mut self, order: Order) {
        if order.size == Decimal::ZERO {
            // Remove existing order.
            if let Ok(idx) = self.search(&order) {
                // Found order with target price.
                self.vec.remove(idx);
            } // It's ok if such order is not found.
        } else {
            // Insert new order or update existing.
            match self.search(&order) {
                Ok(idx) => {
                    // Update existing order.
                    self.vec[idx].size = order.size;
                }
                Err(idx) => {
                    // Maybe insert new order.
                    if idx < self.cap {
                        // We don't want to exceed order book's capacity.
                        if self.vec.len() == self.cap {
                            // If capacity is full, remove last (worst) element
                            // because otherwise it'll be shifted to the right
                            // increasing length beyond capacity.
                            self.vec.pop();
                        }
                        // Insert new order.
                        self.vec.insert(idx, order.clone());
                    }
                }
            }
        }
    }

    fn search(&self, order: &Order) -> Result<usize, usize> {
        if self.rev {
            self.vec.binary_search_by(|o| order.price.cmp(&o.price))
        } else {
            self.vec.binary_search_by(|o| o.price.cmp(&order.price))
        }
    }
}

impl Book {
    pub fn new(cap: usize) -> Self {
        Self {
            bids: Side::new(cap, true),
            asks: Side::new(cap, false),
        }
    }
}
