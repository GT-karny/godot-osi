// GTRM_* shim: road-network topology — road predecessor/successor links,
// junctions, their connections and lane links, and network controllers. The
// stock esminiRMLib C API exposes only junction id-string lookups.
//
// (GT_esmini/.../GT_esminiRMLib.hpp sketches a similar surface; we re-implement
// it self-contained here rather than depending on that module.)
//
// See gt_common.hpp for the shim conventions. We never modify external/esmini.

#include "gt_common.hpp"

#include <string>

using namespace roadmanager;

// A road's predecessor/successor link. `element_type` is RoadLink::ElementType
// (0 unknown,1 road,2 junction); `contact_point` is ContactPointType
// (0 undefined,1 start,2 end,3 junction).
struct GTRM_RoadLink
{
    int          element_type;
    unsigned int element_id;
    int          contact_point;
};

// A junction. `type` is Junction::JunctionType (0 default,1 direct,2 virtual).
struct GTRM_Junction
{
    unsigned int id;
    unsigned int global_id;
    int          type;
    int          n_connections;
    int          n_controllers;
    const char*  name;
};

// One connection within a junction.
struct GTRM_JunctionConnection
{
    unsigned int incoming_road_id;
    unsigned int connecting_road_id;
    int          contact_point;
    int          n_lane_links;
};

// One incoming->connecting lane mapping of a junction connection.
struct GTRM_LaneLink
{
    int from;
    int to;
};

// A network controller (<controller>).
struct GTRM_Controller
{
    unsigned int id;
    int          sequence;
    int          n_controls;
    const char*  name;
};

namespace
{
    std::string g_junction_name;
    std::string g_controller_name;
}  // namespace

extern "C"
{
    // Fill `out` with the predecessor (link_type=-1) or successor (link_type=1)
    // link of `road_id`. Returns 0 if such a link exists, -1 otherwise.
    int GTRM_GetRoadLink(unsigned int road_id, int link_type, GTRM_RoadLink* out)
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
        if (road == nullptr)
        {
            return -1;
        }
        RoadLink* link = road->GetLink(static_cast<LinkType>(link_type));
        if (link == nullptr)
        {
            return -1;
        }
        out->element_type  = static_cast<int>(link->GetElementType());
        out->element_id    = link->GetElementId();
        out->contact_point = static_cast<int>(link->GetContactPointType());
        return 0;
    }

    // Number of junctions in the network; -1 on error.
    int GTRM_GetNumberOfJunctions()
    {
        OpenDrive* odr = gt::odr();
        return odr == nullptr ? -1 : static_cast<int>(odr->GetNumOfJunctions());
    }

    // Fill `out` with junction at vector `index`. 0 / -1. `name` is valid until
    // the next GTRM_GetJunctionByIndex call.
    int GTRM_GetJunctionByIndex(unsigned int index, GTRM_Junction* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr || index >= odr->GetNumOfJunctions())
        {
            return -1;
        }
        Junction* j = odr->GetJunctionByIdx(index);
        if (j == nullptr)
        {
            return -1;
        }
        g_junction_name    = j->GetName();
        out->id            = j->GetId();
        out->global_id     = j->GetGlobalId();
        out->type          = static_cast<int>(j->GetType());
        out->n_connections = static_cast<int>(j->GetNumberOfConnections());
        out->n_controllers = static_cast<int>(j->GetNumberOfControllers());
        out->name          = g_junction_name.c_str();
        return 0;
    }

    // Fill `out` with connection `conn_idx` of junction `junction_id`. 0 / -1.
    int GTRM_GetJunctionConnection(unsigned int junction_id, unsigned int conn_idx, GTRM_JunctionConnection* out)
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
        Junction* j = odr->GetJunctionById(junction_id);
        if (j == nullptr || conn_idx >= j->GetNumberOfConnections())
        {
            return -1;
        }
        Connection* c = j->GetConnectionByIdx(conn_idx);
        if (c == nullptr)
        {
            return -1;
        }
        Road* incoming   = c->GetIncomingRoad();
        Road* connecting = c->GetConnectingRoad();
        out->incoming_road_id   = (incoming != nullptr) ? incoming->GetId() : ID_UNDEFINED;
        out->connecting_road_id = (connecting != nullptr) ? connecting->GetId() : ID_UNDEFINED;
        out->contact_point      = static_cast<int>(c->GetContactPoint());
        out->n_lane_links       = static_cast<int>(c->GetNumberOfLaneLinks());
        return 0;
    }

    // Fill `out` with lane link `link_idx` of connection `conn_idx`. 0 / -1.
    int GTRM_GetJunctionLaneLink(unsigned int junction_id, unsigned int conn_idx, unsigned int link_idx, GTRM_LaneLink* out)
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
        Junction* j = odr->GetJunctionById(junction_id);
        if (j == nullptr || conn_idx >= j->GetNumberOfConnections())
        {
            return -1;
        }
        Connection* c = j->GetConnectionByIdx(conn_idx);
        if (c == nullptr || link_idx >= c->GetNumberOfLaneLinks())
        {
            return -1;
        }
        JunctionLaneLink* ll = c->GetLaneLink(link_idx);
        if (ll == nullptr)
        {
            return -1;
        }
        out->from = ll->from_;
        out->to   = ll->to_;
        return 0;
    }

    // Number of network controllers; -1 on error.
    int GTRM_GetNumberOfControllers()
    {
        OpenDrive* odr = gt::odr();
        return odr == nullptr ? -1 : static_cast<int>(odr->GetNumberOfControllers());
    }

    // Fill `out` with controller at vector `index`. 0 / -1. `name` is valid until
    // the next GTRM_GetController call.
    int GTRM_GetController(unsigned int index, GTRM_Controller* out)
    {
        if (out == nullptr)
        {
            return -1;
        }
        OpenDrive* odr = gt::odr();
        if (odr == nullptr || index >= odr->GetNumberOfControllers())
        {
            return -1;
        }
        Controller* ctrl = odr->GetControllerByIdx(index);
        if (ctrl == nullptr)
        {
            return -1;
        }
        g_controller_name = ctrl->GetName();
        out->id           = ctrl->GetId();
        out->sequence     = ctrl->GetSequence();
        out->n_controls   = static_cast<int>(ctrl->GetNumberOfControls());
        out->name         = g_controller_name.c_str();
        return 0;
    }
}
