use ::std::{env, io};
use langram::{DetectorBuilder, ModelsStorage};

fn main() {
    let models_storage = ModelsStorage::new().unwrap();
    let detector = DetectorBuilder::new(&models_storage).build();

    let mut args = env::args();
    args.next();

    let text = args
        .next()
        .inspect(|t| println!("{}", t))
        .unwrap_or_else(|| {
            let mut text = String::new();
            io::stdin().read_line(&mut text).unwrap();
            text
        });

    let result = detector.detect_top_one_raw(&text);
    println!("detect_top_one_raw {:?}", result);

    let result = detector.detect_top_one_reordered(&text);
    println!("detect_top_one_reordered {:?}", result);

    let mut result = detector.probabilities(&text);
    result.truncate(6);
    println!("probabilities {:?}", result);

    let mut result = detector.probabilities_relative(&text);
    result.truncate(6);
    println!("probabilities_relative {:?}", result);
}
