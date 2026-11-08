// hugecp <src> <dst>: copy a file into hugetlbfs (which has no write(): the destination is sized
// to whole huge pages and filled through a shared mapping). E.g. to put a table made on /data
// back on /mnt/huge. The trailer of a cube-opt table sits at the end of the source; it is
// copied to the end of the (page-rounded) destination, where `solve` looks for it.
#define _GNU_SOURCE
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/vfs.h>
#include <unistd.h>
int main(int argc, char **argv) {
	if (argc != 3) { fprintf(stderr, "usage: hugecp src dst\n"); return 2; }
	int in = open(argv[1], O_RDONLY);
	struct stat st;
	if (in < 0 || fstat(in, &st)) { perror(argv[1]); return 1; }
	int out = open(argv[2], O_RDWR | O_CREAT | O_TRUNC, 0644);
	struct statfs sf;
	if (out < 0 || fstatfs(out, &sf)) { perror(argv[2]); return 1; }
	size_t page = (size_t) sf.f_bsize, len = (size_t) st.st_size;
	size_t rounded = (len + page - 1) / page * page;
	if (ftruncate(out, (off_t) rounded)) { perror("ftruncate"); return 1; }
	char *dst = mmap(NULL, rounded, PROT_READ | PROT_WRITE, MAP_SHARED, out, 0);
	if (dst == MAP_FAILED) { perror("mmap (free pages in the pool?)"); return 1; }
	// A cube-opt table: data, then its 4 KiB trailer at the very end of the file.
	size_t trailer = 4096, data = len;
	static const char legacy[] = "cubeopt h48 table v1\n";
	static const char modern[] = "egg-solver h48 table v1\n";
	char head[sizeof(modern)-1];
	if (len >= trailer && pread(in, head, sizeof(head), (off_t)(len-trailer)) == (ssize_t)sizeof(head) &&
	    (!memcmp(head, legacy, sizeof(legacy)-1) || !memcmp(head, modern, sizeof(modern)-1)))
		data = len-trailer;
	for (size_t off = 0; off < data;) {
		ssize_t r = pread(in, dst + off, data - off < (64 << 20) ? data - off : (64 << 20), (off_t) off);
		if (r <= 0) { perror("read"); return 1; }
		off += (size_t) r;
	}
	if (data != len && pread(in, dst + rounded - trailer, trailer, (off_t) data) != (ssize_t) trailer) {
		perror("read trailer"); return 1;
	}
	if (munmap(dst, rounded) || close(out)) { perror("close"); return 1; }
	printf("%s -> %s: %zu bytes in %zu pages of %zu MiB%s\n", argv[1], argv[2], len, rounded / page,
	       page >> 20, data != len ? " (trailer moved to the end)" : "");
	return 0;
}
