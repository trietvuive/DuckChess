use duck_chess::repr::bitboard::Bitboard;

#[test]
fn test_bitboard_basics() {
    let mut bb = Bitboard::EMPTY;
    assert_eq!(bb.pop_count(), 0);
    
    bb.set_bit(0);
    assert!(bb.is_set(0));
    assert_eq!(bb.pop_count(), 1);
    
    bb.set_bit(63);
    assert!(bb.is_set(63));
    assert_eq!(bb.pop_count(), 2);
    
    bb.clear_bit(0);
    assert!(!bb.is_set(0));
    assert_eq!(bb.pop_count(), 1);
}

#[test]
fn test_bitboard_operations() {
    let bb1 = Bitboard::from_square(0);
    let bb2 = Bitboard::from_square(1);
    
    let or = bb1 | bb2;
    assert_eq!(or.pop_count(), 2);
    
    let and = bb1 & bb2;
    assert_eq!(and.pop_count(), 0);
    
    let xor = bb1 ^ bb2;
    assert_eq!(xor.pop_count(), 2);
}

#[test]
fn test_bitboard_display() {
    let mut bb = Bitboard::EMPTY;
    bb.set_bit(0);  // a1
    bb.set_bit(7);  // h1
    bb.set_bit(56); // a8
    bb.set_bit(63); // h8
    
    let display = format!("{}", bb);
    let lines: Vec<&str> = display.lines().collect();
    
    // Check that we have 8 ranks
    assert_eq!(lines.len(), 8);
    
    // Check that the corners are set
    assert!(lines[0].contains("1")); // h1
    assert!(lines[7].contains("1")); // a8
}

#[test]
fn test_bitboard_constants() {
    assert_eq!(Bitboard::EMPTY.0, 0);
    assert_eq!(Bitboard::FULL.0, 0xFFFFFFFFFFFFFFFF);
}

#[test]
fn test_bitboard_lsb_msb() {
    let bb = Bitboard::from_square(0);
    assert_eq!(bb.lsb(), Some(0));
    assert_eq!(bb.msb(), Some(0));

    let bb = Bitboard::from_square(63);
    assert_eq!(bb.lsb(), Some(63));
    assert_eq!(bb.msb(), Some(63));

    let bb = Bitboard::EMPTY;
    assert_eq!(bb.lsb(), None);
    assert_eq!(bb.msb(), None);

    let mut bb = Bitboard::EMPTY;
    bb.set_bit(0);
    bb.set_bit(63);
    assert_eq!(bb.lsb(), Some(0));
    assert_eq!(bb.msb(), Some(63));
}