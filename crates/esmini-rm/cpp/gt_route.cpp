// GTRM_* shim: network-level routing via RoadManager's RoadPath (Dijkstra
// shortest path between two road positions).
//
// Scope note: Route, RMTrajectory and the Shape classes
// (PolyLine/Clothoid/ClothoidSpline/Nurbs) are populated by the *scenario
// engine* from OpenSCENARIO, not by loading an .xodr through RM_Init. In this
// plugin's road-only flow there is no such data to expose, so only the
// road-network routing (RoadPath) is wrapped here. Network <controller>s are
// already exposed via gt_topology.cpp.
//
// See gt_common.hpp for the shim conventions. We never modify external/esmini.

#include "gt_common.hpp"

using namespace roadmanager;

extern "C"
{
    // Shortest-path distance (m) between (road_a, s_a) and (road_b, s_b),
    // searching both directions, written to `out_dist`. A negative distance
    // means the path runs opposite the start heading. Returns 0 on success,
    // -1 if no path is found or on error.
    int GTRM_ShortestPathDistance(unsigned int road_a, double s_a, unsigned int road_b, double s_b, double* out_dist)
    {
        if (out_dist == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr || odr->GetRoadById(road_a) == nullptr || odr->GetRoadById(road_b) == nullptr)
        {
            return -1;
        }

        Position start;
        Position target;
        start.SetTrackPos(road_a, s_a, 0.0);
        target.SetTrackPos(road_b, s_b, 0.0);

        RoadPath path(&start, &target);
        double   dist = 0.0;
        if (path.Calculate(dist) != 0)
        {
            return -1;
        }
        *out_dist = dist;
        return 0;
    }
}
