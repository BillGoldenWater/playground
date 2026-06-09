use std::{
    fmt::Debug,
    io::{Write, stdout},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Serialize, Deserialize)]
struct Test {
    abcd: Vec<i32>,
    a: i32,
    c: Test2,
    d: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Test2 {
    some: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let v = Test {
        abcd: vec![1, 1, 4, 5, 1, 4],
        a: 1000,
        c: Test2 {
            some: Some("Hello".into()),
        },
        d: None,
    };
    run(&v)?;
    Ok(())
}

fn run<T: Debug + Serialize + DeserializeOwned>(
    v: &T,
) -> anyhow::Result<()> {
    let json_out = serde_json::to_string_pretty(&v)?;

    println!("{json_out}");

    let mut ser = serde_bcbc::ser::Serializer::new();
    v.serialize(&mut ser)?;
    let bcbc_out = ser.into_output();

    for it in bcbc_out {
        if (0x20..=0x7E).contains(&it) {
            stdout().write_all(&[it])?;
        } else {
            print!("\\x{it:0>2x}");
        }
    }
    println!();

    let v_de: T = serde_json::from_str(&json_out)?;
    dbg!(v_de);

    Ok(())
}

/*
R\x03
    D\x02
        Ux
        P\x00
    G\x00
    P\x04
        P\x06
            I\x01
            I\x01
            I\x04
            I\x05
            I\x01
            I\x04
        I\xe8\x07
        R\x03
            D\x02
                Ux
                P\x00
            G\x00
            P\x01
                O\x02
                    F\x01
                    S\x05
                        Hello
        O\x02
            F\x00
            T\x02
                U0
                P\x00
 */
