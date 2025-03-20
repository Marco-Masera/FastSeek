import matplotlib.pyplot as plt
import os

fastseek_index, fastseek_search, samtools_index, samtools_search, fastseek_index_file, samtools_index_file = snakemake.input

def parse_snakemake_benchmark(filename):
    with open(filename) as f:
        lines = f.readlines()
    return float(lines[1].split("\t")[0])

def get_file_size(filename):
    return os.path.getsize(filename) / (1024 ** 2)

# Create a figure with 3 subplots
fig, axs = plt.subplots(1, 3, figsize=(15, 5))

# Plot indexing time
fastseek_indexing_time = parse_snakemake_benchmark(fastseek_index)
samtools_indexing_time = parse_snakemake_benchmark(samtools_index)
indexing_times = [fastseek_indexing_time, samtools_indexing_time]
indexing_labels = ['FastSeek', 'Samtools']
axs[0].bar(indexing_labels, indexing_times, color=['green', 'orange'])
axs[0].set_ylabel('Time (s)')
axs[0].set_title('Indexing time')
for i, v in enumerate(indexing_times):
    axs[0].text(i, v + 0.01, f'{v:.2f} s', ha='center')

# Plot searching time
fastseek_search_time = parse_snakemake_benchmark(fastseek_search)
samtools_search_time = parse_snakemake_benchmark(samtools_search)
search_times = [fastseek_search_time, samtools_search_time]
search_labels = ['FastSeek', 'Samtools']
axs[1].bar(search_labels, search_times, color=['green', 'orange'])
axs[1].set_ylabel('Time (s)')
axs[1].set_yscale('log')
axs[1].set_title('Searching time (log scale)')
for i, v in enumerate(search_times):
    axs[1].text(i, v + 0.01, f'{v:.2f} s', ha='center')

# Plot indexing file size
fastseek_index_file_size = get_file_size(fastseek_index_file)
samtools_index_file_size = get_file_size(samtools_index_file)
index_file_sizes = [fastseek_index_file_size, samtools_index_file_size]
index_file_labels = ['FastSeek', 'Samtools']
axs[2].bar(index_file_labels, index_file_sizes, color=['green', 'orange'])
axs[2].set_ylabel('Size (MB)')
axs[2].set_title('Index file size')
for i, v in enumerate(index_file_sizes):
    axs[2].text(i, v + 0.01, f'{v:.2f} MB', ha='center')

# Save the combined plot
plt.tight_layout()
plt.savefig(snakemake.output[0])
plt.close()
