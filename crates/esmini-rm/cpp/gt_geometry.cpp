// GTRM_* shim: OpenDRIVE reference-line geometry primitives and precomputed OSI
// sample points, neither of which the stock esminiRMLib C API exposes.
//
// See gt_common.hpp for the shim conventions. We never modify external/esmini.

#include "gt_common.hpp"

#include <vector>

using namespace roadmanager;

// --- Reference-line geometry (convention A: record enumeration) -------------

// One <geometry> record of a road's reference line. `type` is
// roadmanager::Geometry::GeometryType (0=unknown,1=line,2=arc,3=spiral,
// 4=poly3,5=paramPoly3). curv_* are meaningful for arc/spiral; a..d / a2..d2
// hold polynomial coefficients for poly3 (a..d) and paramPoly3 (U=a..d, V=a2..d2).
struct GTRM_Geometry
{
    unsigned int road_id;
    int          type;
    double       s;
    double       x;
    double       y;
    double       hdg;
    double       length;
    double       curv_start;
    double       curv_end;
    double       a;
    double       b;
    double       c;
    double       d;
    double       a2;
    double       b2;
    double       c2;
    double       d2;
};

extern "C"
{
    // Number of <geometry> records on the reference line of `road_id`; -1 on error.
    int GTRM_GetNumberOfGeometries(unsigned int road_id)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr)
        {
            return -1;
        }
        return static_cast<int>(road->GetNumberOfGeometries());
    }

    // Fill `out` with geometry `idx` of `road_id`. Returns 0 on success, -1 on error.
    int GTRM_GetGeometry(unsigned int road_id, unsigned int idx, GTRM_Geometry* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr || idx >= road->GetNumberOfGeometries())
        {
            return -1;
        }
        Geometry* g = road->GetGeometry(idx);
        if (g == nullptr)
        {
            return -1;
        }

        *out            = GTRM_Geometry{};
        out->road_id    = road_id;
        out->type       = static_cast<int>(g->GetType());
        out->s          = g->GetS();
        out->x          = g->GetX();
        out->y          = g->GetY();
        out->hdg        = g->GetHdg();
        out->length     = g->GetLength();
        out->curv_start = 0.0;
        out->curv_end   = 0.0;

        switch (g->GetType())
        {
            case Geometry::GEOMETRY_TYPE_ARC:
            {
                Arc* arc        = static_cast<Arc*>(g);
                out->curv_start = arc->GetCurvature();
                out->curv_end   = arc->GetCurvature();
                break;
            }
            case Geometry::GEOMETRY_TYPE_SPIRAL:
            {
                Spiral* sp      = static_cast<Spiral*>(g);
                out->curv_start = sp->GetCurvStart();
                out->curv_end   = sp->GetCurvEnd();
                break;
            }
            case Geometry::GEOMETRY_TYPE_POLY3:
            {
                Poly3*     p3 = static_cast<Poly3*>(g);
                Polynomial pl = p3->GetPoly3();
                out->a        = pl.GetA();
                out->b        = pl.GetB();
                out->c        = pl.GetC();
                out->d        = pl.GetD();
                break;
            }
            case Geometry::GEOMETRY_TYPE_PARAM_POLY3:
            {
                ParamPoly3* pp = static_cast<ParamPoly3*>(g);
                Polynomial  u  = pp->GetPoly3U();
                Polynomial  v  = pp->GetPoly3V();
                out->a         = u.GetA();
                out->b         = u.GetB();
                out->c         = u.GetC();
                out->d         = u.GetD();
                out->a2        = v.GetA();
                out->b2        = v.GetB();
                out->c2        = v.GetC();
                out->d2        = v.GetD();
                break;
            }
            default:
                break;
        }
        return 0;
    }
}

// --- OSI sample points (convention B: variable-length buffer) ---------------

// One precomputed OSI point of a lane / lane-boundary / lane-section reference
// line. `endpoint` flags the end of a contiguous run (e.g. a dash of a broken
// road mark). Mirrors roadmanager::PointStruct.
struct GTRM_OsiPoint
{
    double s;
    double x;
    double y;
    double z;
    double h;
    double p;
    double r;
    double nx;
    double ny;
    int    endpoint;
};

namespace
{
    std::vector<GTRM_OsiPoint> g_osi;

    // Append a roadmanager OSIPoints set to the global buffer.
    void append_osi(OSIPoints* osi)
    {
        if (osi == nullptr)
        {
            return;
        }
        const std::vector<PointStruct>& pts = osi->GetPoints();
        for (const PointStruct& pt : pts)
        {
            g_osi.push_back(GTRM_OsiPoint{pt.s, pt.x, pt.y, pt.z, pt.h, pt.p, pt.r, pt.nx, pt.ny, pt.endpoint ? 1 : 0});
        }
    }
}  // namespace

// OSI-point selectors for GTRM_BuildLaneOsiPoints `kind`.
#define GTRM_OSI_LANE      0  // the lane's own OSI points (outer edge)
#define GTRM_OSI_REF_LINE  1  // the lane section's reference-line OSI points
#define GTRM_OSI_BOUNDARY  2  // the lane's OSI lane-boundary points

extern "C"
{
    // Build the OSI sample points of one lane (or lane-section reference line)
    // into an internal buffer. `kind` is one of GTRM_OSI_*; for REF_LINE the
    // `lane_id` argument is ignored. Returns the number of points, 0 if none.
    int GTRM_BuildLaneOsiPoints(unsigned int road_id, unsigned int section_idx, int lane_id, int kind)
    {
        g_osi.clear();

        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return 0;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr || section_idx >= road->GetNumberOfLaneSections())
        {
            return 0;
        }
        LaneSection* ls = road->GetLaneSectionByIdx(section_idx);
        if (ls == nullptr)
        {
            return 0;
        }

        if (kind == GTRM_OSI_REF_LINE)
        {
            append_osi(&ls->GetRefLineOSIPoints());
            return static_cast<int>(g_osi.size());
        }

        Lane* lane = ls->GetLaneById(lane_id);
        if (lane == nullptr)
        {
            return 0;
        }
        if (kind == GTRM_OSI_BOUNDARY)
        {
            LaneBoundaryOSI* b = lane->GetLaneBoundary();
            if (b != nullptr)
            {
                append_osi(b->GetOSIPoints());
            }
        }
        else  // GTRM_OSI_LANE
        {
            append_osi(lane->GetOSIPoints());
        }
        return static_cast<int>(g_osi.size());
    }

    // Copy the built points: `out` needs room for the count returned by Build.
    void GTRM_CopyOsiPoints(GTRM_OsiPoint* out)
    {
        if (out != nullptr)
        {
            std::copy(g_osi.begin(), g_osi.end(), out);
        }
    }

    // Release the accumulated points.
    void GTRM_ClearOsiPoints()
    {
        std::vector<GTRM_OsiPoint>().swap(g_osi);
    }
}
