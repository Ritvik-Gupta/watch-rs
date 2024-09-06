space() {
    num_lines=$(tput lines);
    for ((i = 1; i <= $(( $num_lines / 3 )); i++ ))
    do
        printf '\n';
    done
}


echo "Running Cargo fix"
cargo fix --all-features --allow-dirty --allow-staged
space


echo "Running Cargo build"
RUST_BACKTRACE=full cargo build

exit_code=$?
if [[ $exit_code != 0 ]]; then
    echo "Encountered error, exiting build." 2>&1
    exit $exit_code
fi
space


echo "Running Cargo release build"
cargo b --release