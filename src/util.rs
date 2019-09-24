
pub(crate) fn strsplit(s: String, delim: char) -> Result<(String,String), &'static str> {
    let opt_pos = s.find(delim);
    match opt_pos {
        None => Err("delim not found"),
        Some(pos) => {
            if s.len() <= 1 {
                Ok(("".to_string(),"".to_string()))
            } else if pos == 0 {
                Ok(("".to_string(), s[1..].to_string()))
            } else if pos == (s.len()-1) {
                Ok((s[0..pos].to_string(), "".to_string()))
            } else {
                Ok((s[0..pos].to_string(), s[(pos+1)..].to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // no delim in string
    #[test]
    fn c0() {
        let r_split = strsplit(String::from("abcd"), ':');
        match r_split {
            Err(_e) => {},
            Ok((_a,_b)) => assert!(false)
        }
    }

    // only delim in string
    #[test]
    fn c1() {
        let r_split = strsplit(String::from(":"), ':');
        match r_split {
            Err(_e) => assert!(false),
            Ok((a,b)) => {
                assert_eq!(a, "");
                assert_eq!(b, "");
            }
        }
    }

    // delim first
    #[test]
    fn c2() {
        let r_split = strsplit(String::from(":abcd"), ':');
        match r_split {
            Err(_e) => assert!(false),
            Ok((a,b)) => {
                assert_eq!(a, "");
                assert_eq!(b, "abcd");
            }
        }
    }

    // delim last
    #[test]
    fn c3() {
        let r_split = strsplit(String::from("abcd:"), ':');
        match r_split {
            Err(_e) => assert!(false),
            Ok((a,b)) => {
                assert_eq!(a, "abcd");
                assert_eq!(b, "");
            }
        }
    }

    // delim in middle, normal case
    #[test]
    fn c4() {
        let r_split = strsplit(String::from("ab:cd"), ':');
        match r_split {
            Err(_e) => assert!(false),
            Ok((a,b)) => {
                assert_eq!(a, "ab");
                assert_eq!(b, "cd");
            }
        }
    }
}
