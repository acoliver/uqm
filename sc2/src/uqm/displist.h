//Copyright Paul Reiche, Fred Ford. 1992-2002

/*
 *  This program is free software; you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation; either version 2 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program; if not, write to the Free Software
 *  Foundation, Inc., 59 Temple Place - Suite 330, Boston, MA 02111-1307, USA.
 */

#ifndef UQM_DISPLIST_H_
#define UQM_DISPLIST_H_

#include <assert.h>
#include "port.h"
#include "libs/compiler.h"
#include "libs/memlib.h"

#if defined(__cplusplus)
extern "C" {
#endif

// The queue is a fixed pool of equally sized slots (pq_tab), allocated up
// front by InitQueue. Certain gameplay elements depend on that: the maximum
// number of HyperSpace encounter globes chasing the player is defined by the
// allocated size of the encounter_q.

typedef void* QUEUE_HANDLE;

typedef UWORD OBJ_SIZE;
typedef QUEUE_HANDLE HLINK;

typedef struct link
{
	// Every queue element of any queue must have these
	// two as the first members
	HLINK pred;
	HLINK succ;
} LINK;

typedef struct /* queue */
{
	HLINK head;
	HLINK tail;
	BYTE  *pq_tab;
	HLINK free_list;
	COUNT object_size;
	BYTE num_objects;
} QUEUE;

extern HLINK AllocLink (QUEUE *pq);
extern void FreeLink (QUEUE *pq, HLINK hLink);

static inline LINK *
LockLink (const QUEUE *pq, HLINK h)
{
	if (h) // Apparently, h==0 is OK
	{	// Make sure the link is actually in our queue!
		assert (pq->pq_tab && (BYTE*)h >= pq->pq_tab &&
				(BYTE*)h < pq->pq_tab + pq->object_size * pq->num_objects);
	}
	return (LINK*)h;
}

static inline void
UnlockLink (const QUEUE *pq, HLINK h)
{
	if (h) // Apparently, h==0 is OK
	{	// Make sure the link is actually in our queue!
		assert (pq->pq_tab && (BYTE*)h >= pq->pq_tab &&
				(BYTE*)h < pq->pq_tab + pq->object_size * pq->num_objects);
	}
}

#define GetLinkSize(pq) (COUNT)((pq)->object_size)
#define GetHeadLink(pq) ((pq)->head)
#define GetTailLink(pq) ((pq)->tail)
#define _GetPredLink(lpE) ((lpE)->pred)
#define _GetSuccLink(lpE) ((lpE)->succ)

extern BOOLEAN InitQueue (QUEUE *pq, COUNT num_elements, OBJ_SIZE size);
extern BOOLEAN UninitQueue (QUEUE *pq);
extern void ReinitQueue (QUEUE *pq);
extern void PutQueue (QUEUE *pq, HLINK hLink);
extern void InsertQueue (QUEUE *pq, HLINK hLink, HLINK hRefLink);
extern void RemoveQueue (QUEUE *pq, HLINK hLink);
extern COUNT CountLinks (QUEUE *pq);
void ForAllLinks(QUEUE *pq, void (*callback)(LINK *, void *), void *arg);

#if defined(__cplusplus)
}
#endif

#endif /* UQM_DISPLIST_H_ */
