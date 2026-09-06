#ifndef API_H
#define API_H

/* Declared here, defined in exactly one file that includes this header. */
int handle(void);

/* Declared here and defined by two files that both include this header, so
   the link is ambiguous and no call to it records an edge. */
int dispatch(void);

/* Declared here, defined by a file that does not include this header, so
   nothing anchors the two together. */
int orphan(void);

#endif
