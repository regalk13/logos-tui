use color_eyre::{
    Result,
    eyre::{WrapErr, eyre},
};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

#[derive(Clone)]
pub struct Verse {
    pub book: String,
    pub chapter: u16,
    pub verse: u16,
    pub text: String,
}

#[derive(Default, Clone)]
pub struct Bible {
    pub verses: Vec<Verse>,
}

impl Bible {
    pub fn load_tsv(path: impl AsRef<Path>) -> Result<Self> {
        let file =
            File::open(&path).wrap_err_with(|| format!("cannot open {:?}", path.as_ref()))?;
        let reader = BufReader::new(file);
        let mut out = Vec::new();

        for (lineno, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            match cols.as_slice() {
                [book, _, ch, vs, txt] => {
                    out.push(Self::make_verse(book, ch, vs, txt, lineno + 1)?);
                }
                [book, _, _book_no, ch, vs, txt] => {
                    out.push(Self::make_verse(book, ch, vs, txt, lineno + 1)?);
                }
                _ => {
                    return Err(eyre!(
                        "line {lineno}: expected 5 or 6 columns, got {}",
                        cols.len()
                    ));
                }
            }
        }
        Ok(Self { verses: out })
    }

    fn make_verse(
        book: &str,
        ch: &str,
        vs: &str,
        txt: &str,
        ln: usize,
    ) -> Result<Verse> {
        Ok(Verse {
            book: book.to_owned(),
            chapter: ch
                .trim()
                .parse()
                .wrap_err_with(|| format!("line {ln}: bad chapter “{ch}”"))?,
            verse: vs
                .trim()
                .parse()
                .wrap_err_with(|| format!("line {ln}: bad verse “{vs}”"))?,
            text: txt.to_owned(),
        })
    }

    pub fn chapters(&self) -> Vec<(String, u16)> {
        use itertools::Itertools;
        self.verses
            .iter()
            .map(|v| (v.book.clone(), v.chapter))
            .unique()
            .collect()
    }

    pub fn passage(&self, book: &str, chap: u16) -> Vec<&Verse> {
        self.verses
            .iter()
            .filter(|v| v.book == book && v.chapter == chap)
            .collect()
    }
}
