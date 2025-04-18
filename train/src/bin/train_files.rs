use ::std::{
    fs,
    fs::File,
    io::BufReader,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use alphabet_detector::{reader::ReadCharsChunks, slang_arr_default, Script, ScriptLanguage};
use cap::Cap;
use clap::Parser;
// #[cfg(not(target_env = "msvc"))]
// use jemallocator::Jemalloc;

// #[cfg(not(target_env = "msvc"))]
// #[global_allocator]
// static ALLOCATOR: Cap<Jemalloc> = Cap::new(Jemalloc, usize::MAX);
// static ALLOCATOR: Jemalloc = Jemalloc;

// #[cfg(target_env = "msvc")]
#[global_allocator]
static ALLOCATOR: Cap<::std::alloc::System> = Cap::new(::std::alloc::System, usize::MAX);

// #[cfg(not(target_env = "msvc"))]
// #[global_allocator]
// static GLOBAL: Jemalloc = Jemalloc;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(short = 'i', required = true)]
    inp: String,

    #[arg(short = 'o', required = true)]
    out: String,
}

const THREADS: usize = 8;
const MEM_LIMIT_SLEEP: usize = 6 * 1024 * 1024 * 1024;
fn main() {
    let args = Args::parse();
    let paths = fs::read_dir(&args.inp).unwrap();
    let pool = threadpool::ThreadPool::new(THREADS);
    let langs_seen = Arc::new(Mutex::new(slang_arr_default::<bool>()));
    let out_path = Path::new(&args.out).to_path_buf();

    for path in paths {
        let out_path = out_path.clone();
        let langs_seen = langs_seen.clone();
        pool.execute(move || {
            let path = path.unwrap();
            let file_name = path.file_name().into_string().unwrap();

            println!("*{}* New", file_name);

            while ALLOCATOR.allocated() > MEM_LIMIT_SLEEP {
                println!(
                    "*{}* Mem allocated: {}MB Sleeping...",
                    file_name,
                    ALLOCATOR.allocated() / (1024 * 1024)
                );
                let time = Duration::from_secs(30);
                thread::sleep(time);
            }
            println!(
                "*{}* Mem allocated: {}MB",
                file_name,
                ALLOCATOR.allocated() / (1024 * 1024)
            );

            {
                let Some(lang) = ScriptLanguage::from_str(&file_name) else {
                    panic!("*{}* Not found lang", file_name);
                };
                {
                    let mut guard = langs_seen.lock().unwrap();
                    let lang_seen = guard.get_mut(lang as usize).unwrap();
                    if *lang_seen {
                        drop(guard);
                        panic!("*{}* Have already seen lang: {:?}", file_name, lang);
                    }
                    *lang_seen = true;
                }

                let script = <Option<Script>>::from(lang);
                let langs = script
                    .map(ScriptLanguage::all_with_script)
                    .unwrap_or_default();
                if langs.len() == 1 {
                    println!("*{}* SKIP single lang {:?} in script", file_name, lang);
                    return;
                }
                // TODO: rm this filter
                /* if !matches!(lang, ScriptLanguage::English) {
                    return;
                } */
                // TODO: rm this filter
                /* if script != Some(Script::Latin) {
                    return;
                } */

                let out_mod_path = out_path.join(lang.into_str());
                if out_mod_path.join("unigrams.encom.br").exists() {
                    println!("*{}* EXISTS {:?}", file_name, lang);
                    return;
                }
                println!("*{}* started {:?}", file_name, lang);

                let file = BufReader::new(File::open(path.path()).expect("open failed"));
                let ch_iter = file.chars_chunks(b'\n').map(|v| (0, v.unwrap()));
                let result =
                    langram_train::create_model_and_write_files(&out_mod_path, ch_iter, lang);
                println!("*{}* done model {:?}", file_name, result);

                /* {
                    let file_path = out_mod_path.join("mod.rs");
                    let mut file = fs::File::create(file_path).unwrap();
                    file.write_all(b"mod unigrams;\nmod bigrams;\nmod trigrams;\nmod quadrigrams;\nmod fivegrams;\n\n")
                        .unwrap();
                    file.write_all(b"pub struct ").unwrap();
                    file.write_all(model_name.as_bytes()).unwrap();
                    file.write_all(b"Model;\n\nimpl crate::Model for ").unwrap();
                    file.write_all(model_name.as_bytes()).unwrap();
                    file.write_all(b"Model {\n").unwrap();
                    file.write_all(
                        b"    #[inline(always)]\n    fn check_unigram(c: char) -> f64 {\n        unigrams::prob(c)\n    }\n",
                    )
                    .unwrap();
                    file.write_all(
                        b"    #[inline(always)]\n    fn check_bigram(g: &[char; 2]) -> f64 {\n        bigrams::prob(g)\n    }\n",
                    )
                    .unwrap();
                    file.write_all(
                        b"    #[inline(always)]\n    fn check_trigram(g: &[char; 3]) -> f64 {\n        trigrams::prob(g)\n    }\n",
                    )
                    .unwrap();
                    file.write_all(
                        b"    #[inline(always)]\n    fn check_quadrigram(g: &[char; 4]) -> f64 {\n        quadrigrams::prob(g)\n    }\n",
                    )
                    .unwrap();
                    file.write_all(
                        b"    #[inline(always)]\n    fn check_fivegram(g: &[char; 5]) -> f64 {\n        fivegrams::prob(g)\n    }\n",
                    )
                    .unwrap();
                    file.write_all(b"}\n").unwrap();
                }

                {
                    let file_path = out_path.join("lib.rs");
                    let mut file = fs::File::options().append(true).open(file_path).unwrap();
                    file.write_all(b"mod ").unwrap();
                    file.write_all(mod_dir.as_bytes()).unwrap();
                    file.write_all(b";\n").unwrap();
                    file.write_all(b"pub use ").unwrap();
                    file.write_all(mod_dir.as_bytes()).unwrap();
                    file.write_all(b"::*;\n").unwrap();
                }

                {
                    let file_path = out_path.join("macros.rs");
                    let mut file = fs::File::options().append(true).open(file_path).unwrap();
                    file.write_all(b"ScriptLanguage::").unwrap();
                    // file.write_all(lang.to_full_dbg().as_bytes()).unwrap();
                    file.write_all(lang.to_string().as_bytes()).unwrap();
                    file.write_all(b" => Some(Box::new(lang_models::").unwrap();
                    file.write_all(model_name.as_bytes()).unwrap();
                    file.write_all(b"Model)),\n").unwrap();
                } */
            }

            println!(
                "*{}* malloc_trim {:?} {:?}MB",
                file_name,
                unsafe { libc::malloc_trim(0) },
                ALLOCATOR.allocated() / (1024 * 1024)
            );
        });
    }

    pool.join();
}
