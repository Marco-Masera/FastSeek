
# Runs tests and benchmarks

rule get_binary:
    output:
        "fastseek"
    shell: 
        """
            cargo build --release
            ln -s target/release/fastseek fastseek
        """


# FastQ test - uncompressed
ORIGINAL_FILE = "benchmark_data/ERR2162313_2.fastq.gz"
NUM_ENTRIES = 1000000 
NUM_LINES = NUM_ENTRIES * 4
NUM_SUCCESSFUL_SEARCHES = 100
NUM_UNSUCCESSFUL_SEARCHES = 100

rule run_test:
    input:
        "benchmark_data/test_fastq",
        "benchmark_data/fastseek_index_benchmark.txt",
        "benchmark_data/fastseek_search_benchmark.txt",
        "benchmark_data/samtools_index_benchmark.txt",
        "benchmark_data/samtools_search_benchmark.txt"

rule get_fastq:
    input:
        ORIGINAL_FILE
    output:
        "benchmark_data/test_fastq"
    shell: """
        set +o pipefail; zcat {input} | head -n {NUM_LINES} > {output}
    """

rule shuffle_sequences_id:
    input:
        "benchmark_data/test_fastq"
    output:
        "benchmark_data/shuffled_ids.txt"
    shell: """
        set +o pipefail; cat {input} | awk 'NR % 4 == 1' | shuf > {output}
    """

rule get_sequences_ids_for_searching_fastseek:
    input:
        "benchmark_data/shuffled_ids.txt"
    output:
        "benchmark_data/sequences_ids_fastseek.txt"
    shell: """
        set +o pipefail; cat {input} | head -n {NUM_SUCCESSFUL_SEARCHES} > {output}
    """

rule get_sequences_ids_for_searching:
    input:
        "benchmark_data/sequences_ids_fastseek.txt"
    output:
        "benchmark_data/sequences_ids.txt"
    shell: """
        cat {input} | awk '{{print substr($0, 2) }}' | awk '{{print $1}}' > {output}
    """

rule fastseek_index:
    input:
        "fastseek",
        "benchmark_data/test_fastq"
    output:
        "benchmark_data/fastseek_index_benchmark.txt",
        "benchmark_data/test_fastq.index"
    benchmark:
        "benchmark_data/fastseek_index_benchmark.txt"
    shell: """
        ./fastseek index-fastq --hashmap-size {NUM_ENTRIES} {input[1]}
    """

rule fastseek_search:
    input:
        "fastseek",
        "benchmark_data/test_fastq",
        "benchmark_data/sequences_ids_fastseek.txt",
        "benchmark_data/test_fastq.index"
    output:
        "benchmark_data/fastseek_search_benchmark.txt"
    benchmark:
        "benchmark_data/fastseek_search_benchmark.txt"
    shell: """
        cat {input[2]} | xargs -I {{}} ./fastseek search {input[1]} "{{}}"
    """

rule samtools_index:
    input:
        "fastseek",
        "benchmark_data/test_fastq"
    output:
        "benchmark_data/samtools_index_benchmark.txt",
        "benchmark_data/test_fastq.fai"
    benchmark:
        "benchmark_data/samtools_index_benchmark.txt"
    shell: """
        samtools faidx --fastq {input[1]}
    """

rule samtools_search:
    input:
        "fastseek",
        "benchmark_data/test_fastq",
        "benchmark_data/sequences_ids.txt",
        "benchmark_data/test_fastq.fai"
    output:
        "benchmark_data/samtools_search_benchmark.txt"
    benchmark:
        "benchmark_data/samtools_search_benchmark.txt"
    shell: """
        set +o pipefail; cat {input[2]} | xargs -I {{}} samtools faidx --fastq --fai-idx {input[3]} {input[1]} "{{}}"
    """

rule plot_benchmarks:
    input:
        fastseek_index="benchmark_data/fastseek_index_benchmark.txt",
        fastseek_search="benchmark_data/fastseek_search_benchmark.txt",
        samtools_index="benchmark_data/samtools_index_benchmark.txt",
        samtools_search="benchmark_data/samtools_search_benchmark.txt",
        fastseek_index_file="benchmark_data/test_fastq.index",
        samtools_index_file="benchmark_data/test_fastq.fai"
    output:
        "benchmark_data/plots.png"
    script:
        "plot_benchmarks.py"