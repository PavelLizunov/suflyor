#include <libproc.h>
#include <stdint.h>
#include <sys/resource.h>

int suflyor_process_footprint(uint32_t pid, uint64_t *bytes) {
    if (pid == 0 || bytes == NULL) {
        return 0;
    }

    struct rusage_info_v4 info = {0};
    if (proc_pid_rusage((int)pid, RUSAGE_INFO_V4, (rusage_info_t *)&info) != 0) {
        return 0;
    }

    *bytes = info.ri_phys_footprint != 0 ? info.ri_phys_footprint : info.ri_resident_size;
    return *bytes != 0;
}
