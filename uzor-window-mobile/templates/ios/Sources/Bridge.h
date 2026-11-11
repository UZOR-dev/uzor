// The Rust side of the host (uzor-window-mobile's C functions and the app entry).
#include <stdint.h>

int32_t __APP___run(void);
void uzor_mobile_set_safe_area(double top, double right, double bottom, double left);
void uzor_mobile_set_keyboard(double height);
void uzor_mobile_set_haptics(void (*perform)(int32_t kind));
