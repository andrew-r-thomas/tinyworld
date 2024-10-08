# notes
## tw file format
**page size**
so the page size is going to be based on the size of the vectors, we basically
always want to be able to get a full vector in a page, also im thinking it 
probably makes sense to keep vectors and index data separate, so we can probably
load the entire index in just a couple of pages. we can start with random access
at first, but i think it also makes sense to keep everything completely separate
(all vectors together in contiguous pages, all index data etc), since we're doing
everything packed together, this might be less of an issue, since like the index data
isn't going to be that big, and we're only getting metadata sometimes,
so maybe we try to keep the index in memory at all times

the expected max index data size is:
number of vecs * (m_max_0 + ())
