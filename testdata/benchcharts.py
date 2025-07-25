import os
import argparse
import json
import matplotlib.pyplot as plt
from matplotlib.gridspec import GridSpec

def main():
    # Parse directory containing benchmark JSON files
    parser = argparse.ArgumentParser(description="Generate benchmark charts from JSON files.")
    parser.add_argument(
        "--input-dir",
        type=str,
        default="testdata/benchmarks",
        help="Path to directory containing benchmark JSON files (default: testdata/benchmarks)"
    )
    args = parser.parse_args()
    input_dir = args.input_dir

    json_files = [f for f in os.listdir(input_dir) if f.endswith(".json")]

    for file_name in json_files:
        file_path = os.path.join(input_dir, file_name)
        save_name = os.path.splitext(file_name)[0] + ".png"
        save_path = os.path.join(input_dir, save_name)

        print(f"Processing: {file_path} → {save_path}")

        with open(file_path) as f:
            data = json.load(f)

        cycle_data = data["cycle_tracker_results"]
        sorted_items = sorted(cycle_data.items(), key=lambda x: x[1], reverse=True)
        labels = [label for label, _ in sorted_items]
        values = [count for _, count in sorted_items]

        total_blobs = data["total_blobs"]
        total_blockexec_inputs = data["total_blockexec_inputs"]
        total_gas = data["total_gas"]
        total_instr = data["total_instruction_count"]
        total_syscall_count = data["total_syscall_count"]

        summary_text = (
            f"Total Blobs in Namespace: {total_blobs:,}\n"
            f"Total EVM block executions: {total_blockexec_inputs:,}\n"
            f"Total Gas: {total_gas:,}\n"
            f"Total Instructions: {total_instr:,}\n"
            f"Total Syscalls: {total_syscall_count:,}"
        )

        fig = plt.figure(figsize=(12, 7))
        gs = GridSpec(2, 1, height_ratios=[4, 1], hspace=0.3)

        ax1 = fig.add_subplot(gs[0])
        bars = ax1.barh(labels, values, color='skyblue')
        ax1.set_xlabel("Cycle Count")
        ax1.set_title(file_name.replace(".json", " — Cycle Tracker Breakdown"))
        ax1.invert_yaxis()
        ax1.grid(axis='x', linestyle='--', alpha=0.5)

        for bar, value in zip(bars, values):
            ax1.text(value + max(values) * 0.01, bar.get_y() + bar.get_height() / 2,
                     f"{value:,}", va='center')

        ax2 = fig.add_subplot(gs[1])
        ax2.axis('off')
        ax2.text(0, 1, summary_text, fontsize=11, va='top', ha='left',
                 linespacing=1.5, fontfamily='monospace')

        plt.savefig(save_path, dpi=300, bbox_inches="tight")
        plt.close(fig)

if __name__ == "__main__":
    main()