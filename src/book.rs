use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    pub price: Decimal,
    pub size: Decimal,
}

#[derive(Debug, Clone)]
pub struct Side<const REV: bool> {
    vec: Vec<Order>,
    cap: usize,
}

#[derive(Debug, Clone)]
pub struct Book {
    pub(crate) bids: Side<true>,
    pub(crate) asks: Side<false>,
}

impl<const REV: bool> Side<REV> {
    fn new(cap: usize) -> Self {
        Self {
            vec: Vec::with_capacity(cap),
            cap,
        }
    }

    pub(crate) fn shot_update(&mut self, orders: Vec<Order>) {
        self.vec = orders;
    }

    pub(crate) fn diff_update(&mut self, order: Order) {
        if order.size == Decimal::ZERO {
            // Remove existing order.
            if let Ok(idx) = self.search(order) {
                // Found order with target price.
                self.vec.remove(idx);
            } // It's ok if such order is not found.
        } else {
            // Insert new order or update existing.
            match self.search(order) {
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
                        self.vec.insert(idx, order);
                    }
                }
            }
        }
    }

    fn search(&self, order: Order) -> Result<usize, usize> {
        if REV {
            self.vec.binary_search_by(|&o| order.price.cmp(&o.price))
        } else {
            self.vec.binary_search_by(|&o| o.price.cmp(&order.price))
        }
    }
}

impl Book {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            bids: Side::new(cap),
            asks: Side::new(cap),
        }
    }

    pub fn capacity(&self) -> usize {
        self.bids.cap
    }
    
    pub fn bids(&self) -> &Vec<Order> {
        &self.bids.vec
    }
    
    pub fn asks(&self) -> &Vec<Order> {
        &self.asks.vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn side() {
        let mut side = Side::<false>::new(3);
        
        let order0_5 = Order { price: dec!(0.5), size: dec!(43.94) };
        let order1 = Order { price: dec!(1.0), size: dec!(11.04) };
        let order1_5 = Order { price: dec!(1.5), size: dec!(98.5) };
        let order2 = Order { price: dec!(2.0), size: dec!(52.3) };
        let order2_5 = Order { price: dec!(2.5), size: dec!(44.0) };
        
        side.diff_update(order2);
        side.diff_update(order1);
        side.diff_update(order0_5);
        side.diff_update(order2_5);
        side.diff_update(order1_5);

        assert_eq!(side.vec, vec![order0_5, order1, order1_5]);
    }
}
