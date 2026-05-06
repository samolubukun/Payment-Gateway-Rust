use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TestCard {
    pub number: &'static str,
    pub cvv: &'static str,
    pub expiry: &'static str,
    pub balance_cents: i64,
    pub usage: &'static str,
}

pub const TEST_CARDS: &[TestCard] = &[
    TestCard {
        number: "4111111111111111",
        cvv: "123",
        expiry: "12/2030",
        balance_cents: 1000000, // $10,000
        usage: "Happy path",
    },
    TestCard {
        number: "4242424242424242",
        cvv: "456",
        expiry: "06/2030",
        balance_cents: 50000, // $500
        usage: "Low balance",
    },
    TestCard {
        number: "5555555555554444",
        cvv: "789",
        expiry: "09/2030",
        balance_cents: 0,
        usage: "Insufficient funds",
    },
    TestCard {
        number: "5105105105105100",
        cvv: "321",
        expiry: "03/2020",
        balance_cents: 500000, // $5,000
        usage: "Expired card",
    },
];

pub fn validate_luhn(card_number: &str) -> bool {
    let mut sum = 0;
    let mut double = false;
    for c in card_number.chars().rev() {
        if let Some(mut digit) = c.to_digit(10) {
            if double {
                digit *= 2;
                if digit > 9 {
                    digit -= 9;
                }
            }
            sum += digit;
            double = !double;
        } else {
            return false;
        }
    }
    sum % 10 == 0
}

pub fn get_card(number: &str) -> Option<&TestCard> {
    TEST_CARDS.iter().find(|c| c.number == number)
}
