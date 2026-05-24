// GTRM_* shim: OpenDRIVE road objects (<object>: barriers, poles, trees,
// buildings, crosswalks, parking spaces, ...), their outlines, and tunnels —
// none of which the stock esminiRMLib C API exposes (it has road signs only).
//
// See gt_common.hpp for the shim conventions. We never modify external/esmini.

#include "gt_common.hpp"

#include <string>
#include <vector>

using namespace roadmanager;

// One <object>. `type` is RMObject::ObjectType; `orientation` RoadObject::
// Orientation (0 positive,1 negative,2 none). (s,t,z_offset,h_offset,pitch,roll)
// are road-relative; (x,y,z,heading) world. `parking_access` is
// ParkingSpace::Access when type is a parking space, else -1.
struct GTRM_RoadObject
{
    unsigned int road_id;
    unsigned int id;
    unsigned int global_id;
    int          type;
    int          orientation;
    double       s;
    double       t;
    double       z_offset;
    double       h_offset;
    double       pitch;
    double       roll;
    double       x;
    double       y;
    double       z;
    double       heading;
    double       length;
    double       width;
    double       height;
    int          parking_access;
    int          n_outlines;
    int          n_repeats;
    const char*  name;
    const char*  type_str;
};

// One <outline> of an object. `fill_type` is Outline::FillType, `contour_type`
// Outline::ContourType.
struct GTRM_OutlineInfo
{
    unsigned int id;
    int          fill_type;
    int          contour_type;
    int          closed;
    int          roof;
    int          n_corners;
};

// One <tunnel>. `type` is Tunnel::Type (0 standard,1 underpass).
struct GTRM_Tunnel
{
    unsigned int road_id;
    unsigned int id;
    int          type;
    double       s;
    double       length;
    double       width;
    double       lighting;
    double       daylight;
    const char*  name;
};

namespace
{
    // Keep the last-returned strings alive for the caller to copy out promptly
    // (same approach as esminiRMLib's single returnString).
    std::string g_obj_name;
    std::string g_obj_type;
    std::string g_tunnel_name;

    // Outline corner positions of the most recent GTRM_BuildObjectOutline call;
    // the per-vertex int attribute is the outline index within the object.
    gt::TriBuf g_outline;

    RMObject* object_of(unsigned int road_id, unsigned int obj_idx)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return nullptr;
        }
        Road* road = odr->GetRoadById(road_id);
        if (road == nullptr || obj_idx >= road->GetNumberOfObjects())
        {
            return nullptr;
        }
        return road->GetRoadObject(obj_idx);
    }
}  // namespace

extern "C"
{
    // Number of <object> records on `road_id`; -1 on error.
    int GTRM_GetNumberOfObjects(unsigned int road_id)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        return road == nullptr ? -1 : static_cast<int>(road->GetNumberOfObjects());
    }

    // Fill `out` with object `obj_idx` of `road_id`. 0 / -1. `name`/`type_str`
    // point at internal storage valid until the next GTRM_GetRoadObject call.
    int GTRM_GetRoadObject(unsigned int road_id, unsigned int obj_idx, GTRM_RoadObject* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        RMObject* o = object_of(road_id, obj_idx);
        if (o == nullptr)
        {
            return -1;
        }
        g_obj_name = o->GetName();
        g_obj_type = o->GetTypeStr();

        out->road_id     = road_id;
        out->id          = o->GetId();
        out->global_id   = o->GetGlobalId();
        out->type        = static_cast<int>(o->GetType());
        out->orientation = static_cast<int>(o->GetOrientation());
        out->s           = o->GetS();
        out->t           = o->GetT();
        out->z_offset    = o->GetZOffset();
        out->h_offset    = o->GetHOffset();
        out->pitch       = o->GetPitch();
        out->roll        = o->GetRoll();
        out->x           = o->GetX();
        out->y           = o->GetY();
        out->z           = o->GetZ();
        out->heading     = o->GetH();
        out->length      = o->GetLength();
        out->width       = o->GetWidth();
        out->height      = o->GetHeight();
        out->parking_access =
            (o->GetType() == RMObject::ObjectType::PARKINGSPACE) ? static_cast<int>(o->GetParkingSpace().GetAccess()) : -1;
        out->n_outlines = static_cast<int>(o->GetNumberOfOutlines());
        out->n_repeats  = static_cast<int>(o->GetNumberOfRepeats());
        out->name       = g_obj_name.c_str();
        out->type_str   = g_obj_type.c_str();
        return 0;
    }

    // Fill `out` with outline `outline_idx` metadata of object `obj_idx`. 0 / -1.
    int GTRM_GetObjectOutlineInfo(unsigned int road_id, unsigned int obj_idx, unsigned int outline_idx, GTRM_OutlineInfo* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        RMObject* o = object_of(road_id, obj_idx);
        if (o == nullptr || outline_idx >= o->GetNumberOfOutlines())
        {
            return -1;
        }
        Outline* ol = o->GetOutline(outline_idx);
        if (ol == nullptr)
        {
            return -1;
        }
        out->id           = ol->id_;
        out->fill_type    = static_cast<int>(ol->fillType_);
        out->contour_type = static_cast<int>(ol->GetCountourType());
        out->closed       = ol->closed_ ? 1 : 0;
        out->roof         = ol->roof_ ? 1 : 0;
        out->n_corners    = static_cast<int>(ol->corner_.size());
        return 0;
    }

    // Build the world-space outline corners of object `obj_idx` into a buffer;
    // each corner's int attribute is its outline index. Returns the corner count.
    int GTRM_BuildObjectOutline(unsigned int road_id, unsigned int obj_idx)
    {
        g_outline.clear();
        RMObject* o = object_of(road_id, obj_idx);
        if (o == nullptr)
        {
            return 0;
        }
        for (unsigned int oi = 0; oi < o->GetNumberOfOutlines(); oi++)
        {
            Outline* ol = o->GetOutline(oi);
            if (ol == nullptr)
            {
                continue;
            }
            for (OutlineCorner* corner : ol->corner_)
            {
                if (corner == nullptr)
                {
                    continue;
                }
                double p[3];
                corner->GetPos(p[0], p[1], p[2]);
                g_outline.push_vertex(p, static_cast<int>(oi));
            }
        }
        return static_cast<int>(g_outline.vertex_count());
    }

    // Copy built outline corners: `out_xyz` = 3*corners f64, `out_outline_idx` = corners i32.
    void GTRM_CopyObjectOutline(double* out_xyz, int* out_outline_idx)
    {
        g_outline.copy_out(out_xyz, out_outline_idx);
    }

    // Release the accumulated outline corners.
    void GTRM_ClearObjectOutline()
    {
        g_outline.release();
    }

    // Number of <tunnel> records on `road_id`; -1 on error.
    int GTRM_GetNumberOfTunnels(unsigned int road_id)
    {
        OpenDrive* odr = gt::odr();
        if (odr == nullptr)
        {
            return -1;
        }
        Road* road = odr->GetRoadById(road_id);
        return road == nullptr ? -1 : static_cast<int>(road->GetNumberOfTunnels());
    }

    // Fill `out` with tunnel `idx` of `road_id`. 0 / -1.
    int GTRM_GetTunnel(unsigned int road_id, unsigned int idx, GTRM_Tunnel* out)
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
        if (road == nullptr || idx >= road->GetNumberOfTunnels())
        {
            return -1;
        }
        Tunnel* t = road->GetTunnel(idx);
        if (t == nullptr)
        {
            return -1;
        }
        g_tunnel_name  = t->name_;
        out->road_id   = road_id;
        out->id        = t->id_;
        out->type      = static_cast<int>(t->type_);
        out->s         = t->s_;
        out->length    = t->length_;
        out->width     = t->width_;
        out->lighting  = t->lighting_;
        out->daylight  = t->daylight_;
        out->name      = g_tunnel_name.c_str();
        return 0;
    }
}
