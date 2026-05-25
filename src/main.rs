fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_test_passes() {
        assert!(true);
    }
}
