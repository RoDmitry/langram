use ::std::{
    fs,
    fs::{DirEntry, File},
    io::BufReader,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use alphabet_detector::{
    reader::ReadCharsChunks, slang_arr_default, ScriptLanguage, ScriptLanguageArr, UcdScript,
};
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
// 6gb of sleep limit means approx you have at least 14gb
const MEM_LIMIT_SLEEP: usize = 6 * 1024 * 1024 * 1024;

fn process(path: DirEntry, langs_seen: Arc<Mutex<ScriptLanguageArr<bool>>>, out_path: PathBuf) {
    let file_name = path.file_name().into_string().unwrap();
    println!("*{}* New", file_name);

    while ALLOCATOR.allocated() > MEM_LIMIT_SLEEP {
        println!(
            "*{}* Mem allocated: {}MB Sleeping...",
            file_name,
            ALLOCATOR.allocated() / (1024 * 1024)
        );
        thread::sleep(Duration::from_secs(15));
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

        let script = UcdScript::from(lang);
        let langs = ScriptLanguage::all_with_script(script);
        if langs.len() == 1 {
            println!("*{}* SKIP single lang {:?} in script", file_name, lang);
            return;
        }
        // TODO: rm this filter
        /* if !matches!(lang, ScriptLanguage::English) {
            return;
        } */
        // TODO: rm this filter
        /* if script != UcdScript::Latin {
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
        let result = langram_train::create_model_and_write_files(&out_mod_path, ch_iter, lang);
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
}

fn main() {
    let args = Args::parse();
    let paths = fs::read_dir(&args.inp).unwrap();
    let mut pool = threadpool::ThreadPool::new(THREADS);
    let langs_seen = Arc::new(Mutex::new(slang_arr_default::<bool>()));
    let out_path = Path::new(&args.out).to_path_buf();

    let mut files: Vec<_> = paths.map(|p| p.unwrap()).collect();
    files.sort_unstable_by(|a, b| {
        a.metadata()
            .expect("no metadata")
            .len()
            .cmp(&b.metadata().expect("no metadata").len())
    });

    for file_path in files {
        let file_size = file_path.metadata().expect("no metadata").len();
        let max_threads = (MEM_LIMIT_SLEEP / (file_size as usize * 5)).max(1);
        if max_threads < pool.max_count() {
            while pool.queued_count() > 0 || pool.active_count() > max_threads {
                thread::sleep(Duration::from_secs(5));
            }
            println!("Limiting num threads to {max_threads}");
            pool.set_num_threads(max_threads);
        }

        let out_path = out_path.clone();
        let langs_seen = langs_seen.clone();
        pool.execute(move || process(file_path, langs_seen, out_path));
    }

    pool.join();
}
