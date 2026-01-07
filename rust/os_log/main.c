#include <os/log.h>

void rust_os_log(os_log_t log, const char* message) {
    os_log(log, "%{public}s", message);
}

void rust_os_log_error(os_log_t log, const char* message) {
    os_log_error(log, "%{public}s", message);
}
