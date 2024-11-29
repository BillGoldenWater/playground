let min = 0
let max = 10

let rec number_gussing the_number =
  Printf.printf "Input your guess: ";

  let guessed = read_int_opt () in
  if Option.is_none guessed then (
    print_endline "Invalid input!";
    number_gussing the_number)
  else
    let guessed = Option.get guessed in
    if guessed = the_number then print_endline "Correct!\n"
    else if guessed > the_number then (
      print_endline "Too large!";
      number_gussing the_number)
    else (
      print_endline "Too low!";
      number_gussing the_number)

let () =
  Random.self_init ();

  Printf.printf "The number is between %d and %d (include both)\n" min max;
  number_gussing (Random.int_in_range ~min ~max)
