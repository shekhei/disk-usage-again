

#[cfg(test)]
mod basic_integration_tests {
    use dualib::app;
    use std::process::Command;
    static EXEC_NAME: &'static str = "./target/debug/dua";
    
    macro_rules! assert_vu8_str_eq {
    ( $x:expr, $y:expr ) => {
        {
            assert_eq!(std::str::from_utf8($x).unwrap(), $y)
        }
    };
}

    #[test]
    fn given_summarize_and_depth_options_then_it_should_fail() {
        let mut output = Vec::<u8>::new();
        let mut error = Vec::<u8>::new();
        match app::app(&vec![EXEC_NAME, "-s", "-d", "0", "../test-data"], &mut output, &mut error) {
            Ok(_) => unreachable!(),
            Err(x) => assert_eq!(x, 1)
        };
        assert_vu8_str_eq!(&error, "depth and summarize cannot be used together\n");
    }

    #[test]
    fn given_summarize_option_with_one_entry_then_it_should_be_the_same_as_depth_0() {
        let mut output = Vec::<u8>::new();
        let mut error = Vec::<u8>::new();
        let mut output2 = Vec::<u8>::new();
        let mut error2 = Vec::<u8>::new();
        app::app(&vec![EXEC_NAME, "-s", "../test-data"], &mut output, &mut error).unwrap();
        app::app(&vec![EXEC_NAME, "-d", "0", "../test-data"], &mut output2, &mut error2).unwrap();
        assert_eq!(output, output2);
        assert_vu8_str_eq!(&error, "");
    }

    #[test]
    fn given_summarize_option_with_one_entry_then_it_should_only_return_one_result() {
        let mut output = Vec::<u8>::new();
        let mut error = Vec::<u8>::new();
        println!("{:?}", std::env::current_dir().unwrap());
        app::app(&vec![EXEC_NAME, "../test-data", "-d", "0"], &mut output, &mut error).unwrap();
        assert_vu8_str_eq!(&output, "");
        assert_vu8_str_eq!(&error, "");
    }
}
