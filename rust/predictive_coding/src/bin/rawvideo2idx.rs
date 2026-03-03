use predictive_coding::idx_data::IdxData;

fn main() {
    let mut dims = [0, 18, 24];
    let data = std::fs::read("./inputs/badapple/o.bin").unwrap();

    let frame_size = dims[1..].iter().product::<usize>();
    assert!(data.len().is_multiple_of(frame_size));
    let frame_count = data.len() / frame_size;
    dims[0] = frame_count;

    let data = IdxData::new(dims.into(), data);
    data.save("./inputs/badapple/badapple-frames-idx3-ubyte");
}
