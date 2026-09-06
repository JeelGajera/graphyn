/* Defines orphan() without including the header that declares it, so no
   agreement between the two files exists to key the link on. */
int orphan(void) {
    return 4;
}
