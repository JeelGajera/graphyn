#include "api.h"

/* A second definition of dispatch, which is what makes it ambiguous. */
int dispatch(void) {
    return 3;
}
